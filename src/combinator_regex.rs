use std::fmt;
use std::fmt::Debug;

/// A trait for Regex combinators. The key to combinators is a shared interface.
///
/// This interface allows for `O(NM)` regex parsing. It took me a few attempts to find it. My first
/// attempts would have served as parser combinator interfaces, which as far as I could tell do
/// _not_ allow for `O(NM)` regex parsing.
pub trait Regex {
    /// The state of a regex, as it is parses an input.
    type State: RegexState;

    /// Construct the initial, _empty_ state. In NFA terms, this is an empty set of states.
    fn init_state(&self) -> Self::State;

    /// Does the input match this regex? Note that this is not looking for an occurrence of the
    /// Regex pattern _somewhere_ in the input; it's specifically checking that the _entire input_
    /// matches the regex.
    fn is_match(&self, input: &str) -> bool {
        let mut state = self.init_state();
        state.start();
        for ch in input.chars() {
            state.advance(ch);
            match state.accepts() {
                Accepts::Never => return false,
                Accepts::Always => return true,
                _ => (),
            }
        }
        state.accepts().as_bool()
    }
}

/// The state of a regex, as it is parses an input. This `State` can be thought of as the _set_ of
/// NFA states, but it doesn't have to be. It just has to obey this spec:
///
/// **Definition.** At any time, this state is "tracking" a set of strings:
///
/// - The state constructed by `Regex::init_state()` tracks an empty set of strings.
/// - The `start()` method adds the empty string to the tracking set.
/// - The `advance(char)` method appends the char to each string in the tracking set.
///
/// **Requirements.**
///
/// - The `accepts()` method returns `Yes` or `Always` if this regex accepts any of the strings
/// in its tracking set, and `No` or `Never` otherwise.
/// - If it returns `Always`, then its tracking set is guaranteed to contain an accepted string
/// forever, under all possible sequences of `advance`s. This enables short-circuiting
/// optimizations.
/// - If it returns `Never`, then its tracking set will _never_ contain an accepted string. This
/// enables short-circuiting optimizations.
pub trait RegexState: Debug {
    fn start(&mut self);
    fn advance(&mut self, ch: char);
    fn accepts(&self) -> Accepts;
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Accepts {
    /// At least one of the states in the NFA state set is accepting.
    Yes,
    /// None of the states
    No,
    Always,
    Never,
}

impl Accepts {
    fn as_bool(self) -> bool {
        use Accepts::*;

        match self {
            Yes | Always => true,
            No | Never => false,
        }
    }

    fn or(self, other: Accepts) -> Accepts {
        use Accepts::*;

        match (self, other) {
            (Always, _) | (_, Always) => Always,
            (Yes, _) | (_, Yes) => Yes,
            (No, _) | (_, No) => No,
            (Never, Never) => Never,
        }
    }
}

/***************/
/* SimpleState */
/***************/

#[derive(Debug, Clone, Copy)]
enum SimpleState {
    Start,
    End,
    Both,
    Neither,
}

impl SimpleState {
    fn new() -> SimpleState {
        SimpleState::Neither
    }

    fn start(&mut self) {
        use SimpleState::*;

        *self = match *self {
            Neither | Start => Start,
            Both | End => Both,
        }
    }

    fn advance(&mut self) {
        use SimpleState::*;

        *self = match *self {
            Neither | End => Neither,
            Both | Start => End,
        }
    }

    fn die(&mut self) {
        *self = SimpleState::Neither;
    }

    fn accepts(&self) -> Accepts {
        use Accepts::*;
        use SimpleState::*;

        match *self {
            End | Both => Yes,
            Start => No,
            Neither => Never,
        }
    }
}

/*********/
/* Empty */
/*********/

struct Empty;

#[derive(Debug)]
struct EmptyState {
    empty: bool,
}

impl Regex for Empty {
    type State = EmptyState;

    fn init_state(&self) -> EmptyState {
        EmptyState { empty: true }
    }
}

impl RegexState for EmptyState {
    fn start(&mut self) {
        self.empty = true;
    }

    fn advance(&mut self, _: char) {
        self.empty = false;
    }

    fn accepts(&self) -> Accepts {
        use Accepts::*;

        if self.empty {
            Yes
        } else {
            Never
        }
    }
}

/*******/
/* Dot */
/*******/

struct Dot;

#[derive(Debug)]
struct DotState(SimpleState);

impl Regex for Dot {
    type State = DotState;

    fn init_state(&self) -> DotState {
        DotState(SimpleState::new())
    }
}

impl RegexState for DotState {
    fn start(&mut self) {
        self.0.start();
    }

    fn advance(&mut self, _: char) {
        self.0.advance();
    }

    fn accepts(&self) -> Accepts {
        self.0.accepts()
    }
}

/********/
/* Char */
/********/

struct Char(char);

#[derive(Debug)]
struct CharState {
    ch: char,
    state: SimpleState,
}

impl Regex for Char {
    type State = CharState;

    fn init_state(&self) -> CharState {
        CharState {
            ch: self.0,
            state: SimpleState::new(),
        }
    }
}

impl RegexState for CharState {
    fn start(&mut self) {
        self.state.start();
    }

    fn advance(&mut self, ch: char) {
        if ch == self.ch {
            self.state.advance();
        } else {
            self.state.die();
        }
    }

    fn accepts(&self) -> Accepts {
        self.state.accepts()
    }
}

/************/
/* CharFrom */
/************/

struct CharFrom(String);

#[derive(Debug)]
struct CharFromState {
    charset: String,
    state: SimpleState,
}

impl Regex for CharFrom {
    type State = CharFromState;

    fn init_state(&self) -> CharFromState {
        CharFromState {
            charset: self.0.clone(),
            state: SimpleState::new(),
        }
    }
}

impl RegexState for CharFromState {
    fn start(&mut self) {
        self.state.start();
    }

    fn advance(&mut self, ch: char) {
        if self.charset.contains(ch) {
            self.state.advance();
        } else {
            self.state.die();
        }
    }

    fn accepts(&self) -> Accepts {
        self.state.accepts()
    }
}

/********/
/* Star */
/********/

struct Star<P: Regex>(P);

struct StarState<P: Regex> {
    init: bool,
    state: P::State,
}

impl<P: Regex> Debug for StarState<P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("StarState")
            .field("accepts", &self.accepts())
            .field("init", &self.init)
            .field("state", &self.state)
            .finish()
    }
}

impl<P: Regex> Regex for Star<P> {
    type State = StarState<P>;

    fn init_state(&self) -> StarState<P> {
        StarState {
            init: false,
            state: self.0.init_state(),
        }
    }
}

impl<P: Regex> RegexState for StarState<P> {
    fn start(&mut self) {
        self.init = true;
        self.state.start();
    }

    fn advance(&mut self, ch: char) {
        self.init = false;
        self.state.advance(ch);
        if self.state.accepts().as_bool() {
            self.init = true;
            self.state.start();
        }
    }

    fn accepts(&self) -> Accepts {
        use Accepts::*;

        if self.init {
            Yes
        } else {
            self.state.accepts()
        }
    }
}

/*********/
/* Maybe */
/*********/

struct Maybe<P: Regex>(P);

struct MaybeState<P: Regex> {
    init: bool,
    state: P::State,
}

impl<P: Regex> Debug for MaybeState<P> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Maybe")
            .field("accepts", &self.accepts())
            .field("init", &self.init)
            .field("state", &self.state)
            .finish()
    }
}

impl<P: Regex> Regex for Maybe<P> {
    type State = MaybeState<P>;

    fn init_state(&self) -> MaybeState<P> {
        MaybeState {
            init: false,
            state: self.0.init_state(),
        }
    }
}

impl<P: Regex> RegexState for MaybeState<P> {
    fn start(&mut self) {
        self.init = true;
        self.state.start();
    }

    fn advance(&mut self, ch: char) {
        self.init = false;
        self.state.advance(ch);
    }

    fn accepts(&self) -> Accepts {
        if self.init {
            Accepts::Yes
        } else {
            self.state.accepts()
        }
    }
}

/*******/
/* Alt */
/*******/

struct Alt<P: Regex, Q: Regex>(P, Q);

struct AltState<P: Regex, Q: Regex>(P::State, Q::State);

impl<P: Regex, Q: Regex> Debug for AltState<P, Q> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("AltState")
            .field("accepts", &self.accepts())
            .field("left", &self.0)
            .field("right", &self.1)
            .finish()
    }
}

impl<P: Regex, Q: Regex> Regex for Alt<P, Q> {
    type State = AltState<P, Q>;

    fn init_state(&self) -> AltState<P, Q> {
        AltState(self.0.init_state(), self.1.init_state())
    }
}

impl<P: Regex, Q: Regex> RegexState for AltState<P, Q> {
    fn start(&mut self) {
        self.0.start();
        self.1.start();
    }

    fn advance(&mut self, ch: char) {
        self.0.advance(ch);
        self.1.advance(ch);
    }

    fn accepts(&self) -> Accepts {
        self.0.accepts().or(self.1.accepts())
    }
}

/*******/
/* Seq */
/*******/

struct Seq<P: Regex, Q: Regex>(P, Q);

struct SeqState<P: Regex, Q: Regex>(P::State, Q::State);

impl<P: Regex, Q: Regex> Debug for SeqState<P, Q> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("SeqState")
            .field("accepts", &self.accepts())
            .field("first", &self.0)
            .field("second", &self.1)
            .finish()
    }
}

impl<P: Regex, Q: Regex> Regex for Seq<P, Q> {
    type State = SeqState<P, Q>;

    fn init_state(&self) -> SeqState<P, Q> {
        SeqState(self.0.init_state(), self.1.init_state())
    }
}

impl<P: Regex, Q: Regex> RegexState for SeqState<P, Q> {
    fn start(&mut self) {
        self.0.start();
        if self.0.accepts().as_bool() {
            self.1.start();
        }
    }

    fn advance(&mut self, ch: char) {
        self.1.advance(ch);
        self.0.advance(ch);
        if self.0.accepts().as_bool() {
            self.1.start();
        }
    }

    fn accepts(&self) -> Accepts {
        self.1.accepts()
    }
}

pub mod combinators {
    use super::*;

    pub fn dot() -> impl Regex {
        Dot
    }

    pub fn single_char(ch: char) -> impl Regex {
        Char(ch)
    }

    pub fn empty() -> impl Regex {
        Empty
    }

    pub fn one(ch: char) -> impl Regex {
        Char(ch)
    }

    pub fn oneof(charset: &str) -> impl Regex {
        CharFrom(charset.to_owned())
    }

    pub fn seq(first: impl Regex, second: impl Regex) -> impl Regex {
        Seq(first, second)
    }

    pub fn alt(left: impl Regex, right: impl Regex) -> impl Regex {
        Alt(left, right)
    }

    pub fn star(regex: impl Regex) -> impl Regex {
        Star(regex)
    }

    pub fn maybe(regex: impl Regex) -> impl Regex {
        Maybe(regex)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;

    const ANUM: &str = "100100010100010010.10010101000100111";
    const NOTANUM: &str = "100100010100010010.100101010001001.11";

    #[test]
    fn tests() {
        use combinators::*;

        let zero = one('0');
        assert!(!zero.is_match(""));
        assert!(zero.is_match("0"));
        assert!(!zero.is_match("1"));
        assert!(!zero.is_match("00"));
        assert!(!zero.is_match("01"));
        assert!(!zero.is_match("10"));

        let digit = oneof("01");
        assert!(!digit.is_match(""));
        assert!(digit.is_match("0"));
        assert!(digit.is_match("1"));
        assert!(!digit.is_match("2"));
        assert!(!digit.is_match("01"));
        assert!(!digit.is_match("00"));

        let zeroes = star(one('0'));
        assert!(zeroes.is_match(""));
        assert!(zeroes.is_match("0"));
        assert!(zeroes.is_match("00"));
        assert!(!zeroes.is_match("1"));
        assert!(!zeroes.is_match("01"));
        assert!(!zeroes.is_match("0010"));

        let oh_one = seq(one('0'), one('1'));
        assert!(oh_one.is_match("01"));

        let integer = alt(one('0'), seq(one('1'), star(oneof("01"))));
        assert!(integer.is_match("0"));
        assert!(!integer.is_match("2"));
        assert!(integer.is_match("10"));
        assert!(!integer.is_match("01"));
        assert!(integer.is_match("1101001"));
        assert!(!integer.is_match("0101001"));
        assert!(!integer.is_match("1101021"));
    }

    #[bench]
    fn this_crate(bencher: &mut Bencher) {
        use combinators::*;

        let integer = alt(one('0'), seq(one('1'), star(oneof("01"))));
        let tail = seq(one('.'), star(oneof("01")));
        let decimal = seq(integer, maybe(tail));

        bencher.iter(|| {
            assert!(decimal.is_match(ANUM));
            assert!(!decimal.is_match(NOTANUM));
        });
    }

    // Burnt Sushi's Regexes.
    // It's 6 times faster on this example.
    #[bench]
    fn regex_crate(bencher: &mut Bencher) {
        use regex::Regex;
        let number = Regex::new("^(0|1[01]*)(\\.[01]*)?$").unwrap();
        bencher.iter(|| {
            assert!(number.is_match(ANUM));
            assert!(!number.is_match(NOTANUM));
        })
    }
}
