/// A trait for Regex combinators. The key to combinators is a shared interface.
///
/// This interface allows for `O(NM)` regex parsing. It took me a few attempts to find it. My first
/// attempts would have served as parser combinator interfaces, which as far as I could tell do
/// _not_ allow for `O(NM)` regex parsing.
///
/// Users only need call the `Regex.is_match(&str)` method.
///
/// # Spec
///
/// A `Regex` needs to maintain state as it runs. You _can_ think of this `State` as the _set_ of
/// NFA states, but you don't _have_ to. It just has to obey this spec:
///
/// **Definition.** At any time, this state is "tracking" a set of strings:
///
/// - The state constructed by `Regex::init_state()` tracks an empty set of strings.
/// - The `start()` method adds the empty string to the tracking set.
/// - The `advance(u8)` method appends the char to each string in the tracking set.
///
/// **Requirement.** The `accepts()` method returns true iff the `Regex` accepts any of the strings
/// in its tracking set.
pub trait Regex: Clone {
    /// Reset to the initial, _empty_ state. In NFA terms, this is an empty set of states.
    fn initialize(&mut self);
    /// Track an empty string.
    fn start(&mut self);
    /// Append `byte` to every string being tracked.
    fn advance(&mut self, byte: u8);
    /// Does the regex match any of the tracked strings?
    fn accepts(&self) -> bool;
    /// Is it true that both (i) accepts() is false, and (ii) accepts() will remain false for any
    /// possible sequence of `advance`s? This is used for a short-circuiting optimization.
    fn is_dead(&self) -> bool;

    /// Does the input match this regex? Note that this is not looking for an occurrence of the
    /// Regex pattern _somewhere_ in the input; it's specifically checking that the _entire input_
    /// matches the regex.
    fn is_match(&mut self, input: &str) -> bool {
        self.initialize();
        self.start();
        for byte in input.bytes() {
            self.advance(byte);
            if self.is_dead() {
                return false;
            }
        }
        self.accepts()
    }
}

/*******************/
/* Char Predicates */
/*******************/

trait Predicate: Copy {
    fn matches(&self, byte: u8) -> bool;
}

#[derive(Clone, Copy)]
struct SingleChar<P: Predicate> {
    predicate: P,
    state: SimpleState,
}

impl<P: Predicate> SingleChar<P> {
    fn new(predicate: P) -> SingleChar<P> {
        SingleChar {
            predicate,
            state: SimpleState::Neither,
        }
    }
}

impl<P: Predicate> Regex for SingleChar<P> {
    fn initialize(&mut self) {
        self.state = SimpleState::Neither;
    }

    fn start(&mut self) {
        use SimpleState::*;

        self.state = match self.state {
            Neither | Start => Start,
            Both | End => Both,
        }
    }

    fn advance(&mut self, byte: u8) {
        use SimpleState::*;

        if self.predicate.matches(byte) {
            self.state = match self.state {
                Neither | End => Neither,
                Both | Start => End,
            };
        } else {
            self.state = Neither;
        }
    }

    fn accepts(&self) -> bool {
        use SimpleState::*;

        match self.state {
            End | Both => true,
            Start | Neither => false,
        }
    }

    fn is_dead(&self) -> bool {
        self.state == SimpleState::Neither
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SimpleState {
    Start,
    End,
    Both,
    Neither,
}

/***********************/
/* Single Char Regexes */
/***********************/

#[derive(Clone, Copy)]
struct Dot;

impl Predicate for Dot {
    fn matches(&self, _byte: u8) -> bool {
        true
    }
}

#[derive(Clone, Copy)]
struct Byte(u8);

impl Predicate for Byte {
    fn matches(&self, byte: u8) -> bool {
        self.0 == byte
    }
}

#[derive(Clone, Copy)]
struct ByteRange(u8, u8);

impl Predicate for ByteRange {
    fn matches(&self, byte: u8) -> bool {
        self.0 <= byte && byte <= self.1
    }
}

/*********/
/* Empty */
/*********/

#[derive(Clone, Copy)]
struct Empty {
    empty: bool,
}

impl Empty {
    fn new() -> Empty {
        Empty { empty: false }
    }
}

impl Regex for Empty {
    fn initialize(&mut self) {
        self.empty = false;
    }

    fn start(&mut self) {
        self.empty = true;
    }

    fn advance(&mut self, _: u8) {
        self.empty = false;
    }

    fn accepts(&self) -> bool {
        self.empty
    }

    fn is_dead(&self) -> bool {
        !self.empty
    }
}

/********/
/* Star */
/********/

#[derive(Clone)]
struct Star<P: Regex> {
    init: bool,
    state: P,
}

impl<P: Regex> Star<P> {
    fn new(regex: P) -> Star<P> {
        Star {
            init: false,
            state: regex,
        }
    }
}

impl<P: Regex> Regex for Star<P> {
    fn initialize(&mut self) {
        self.init = false;
        self.state.initialize();
    }

    fn start(&mut self) {
        self.init = true;
        self.state.start();
    }

    fn advance(&mut self, byte: u8) {
        self.init = false;
        self.state.advance(byte);
        if self.state.accepts() {
            self.init = true;
            self.state.start();
        }
    }

    fn accepts(&self) -> bool {
        self.init || self.state.accepts()
    }

    fn is_dead(&self) -> bool {
        !self.init && self.state.is_dead()
    }
}

/*********/
/* Maybe */
/*********/

#[derive(Clone)]
struct Maybe<P: Regex> {
    init: bool,
    state: P,
}

impl<P: Regex> Maybe<P> {
    fn new(regex: P) -> Maybe<P> {
        Maybe {
            init: false,
            state: regex,
        }
    }
}

impl<P: Regex> Regex for Maybe<P> {
    fn initialize(&mut self) {
        self.init = false;
        self.state.initialize();
    }

    fn start(&mut self) {
        self.init = true;
        self.state.start();
    }

    fn advance(&mut self, byte: u8) {
        self.init = false;
        self.state.advance(byte);
    }

    fn accepts(&self) -> bool {
        self.init || self.state.accepts()
    }

    fn is_dead(&self) -> bool {
        !self.init && self.state.is_dead()
    }
}

/*******/
/* Alt */
/*******/

#[derive(Clone)]
struct Alt<P: Regex, Q: Regex>(P, Q);

impl<P: Regex, Q: Regex> Regex for Alt<P, Q> {
    fn initialize(&mut self) {
        self.0.initialize();
        self.1.initialize();
    }

    fn start(&mut self) {
        self.0.start();
        self.1.start();
    }

    fn advance(&mut self, byte: u8) {
        self.0.advance(byte);
        self.1.advance(byte);
    }

    fn accepts(&self) -> bool {
        self.0.accepts() || self.1.accepts()
    }

    fn is_dead(&self) -> bool {
        self.0.is_dead() && self.1.is_dead()
    }
}

/*******/
/* Seq */
/*******/

#[derive(Clone)]
struct Seq<P: Regex, Q: Regex>(P, Q);

impl<P: Regex, Q: Regex> Regex for Seq<P, Q> {
    fn initialize(&mut self) {
        self.0.initialize();
        self.1.initialize();
    }

    fn start(&mut self) {
        self.0.start();
        if self.0.accepts() {
            self.1.start();
        }
    }

    fn advance(&mut self, byte: u8) {
        self.1.advance(byte);
        self.0.advance(byte);
        if self.0.accepts() {
            self.1.start();
        }
    }

    fn accepts(&self) -> bool {
        self.1.accepts()
    }

    fn is_dead(&self) -> bool {
        self.0.is_dead() && self.1.is_dead()
    }
}

pub mod combinators {
    use super::*;

    pub fn empty() -> impl Regex {
        Empty::new()
    }

    pub fn dot() -> impl Regex {
        SingleChar::new(Dot)
    }

    pub fn byte(ch: char) -> impl Regex {
        if !ch.is_ascii() {
            panic!("Char does not fit in a byte: {}", ch);
        }
        SingleChar::new(Byte(ch as u8))
    }

    pub fn byte_range(min_ch: char, max_ch: char) -> impl Regex {
        if !min_ch.is_ascii() {
            panic!("Char does not fit in a byte: {}", min_ch);
        }
        if !max_ch.is_ascii() {
            panic!("Char does not fit in a byte: {}", max_ch);
        }
        SingleChar::new(ByteRange(min_ch as u8, max_ch as u8))
    }

    pub fn seq(first: impl Regex, second: impl Regex) -> impl Regex {
        Seq(first, second)
    }

    pub fn alt(left: impl Regex, right: impl Regex) -> impl Regex {
        Alt(left, right)
    }

    pub fn star(regex: impl Regex) -> impl Regex {
        Star::new(regex)
    }

    pub fn maybe(regex: impl Regex) -> impl Regex {
        Maybe::new(regex)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test::Bencher;

    const ANUM: &str = "31415926535897932384626.4338327950288419716939937";
    const NOTANUM: &str = "31415926535897932384626.4338327.95028841971693993";

    #[test]
    fn tests() {
        use combinators::*;

        let mut zero = byte('0');
        assert!(!zero.is_match(""));
        assert!(zero.is_match("0"));
        assert!(!zero.is_match("1"));
        assert!(!zero.is_match("00"));
        assert!(!zero.is_match("01"));
        assert!(!zero.is_match("10"));

        let mut digit = byte_range('0', '1');
        assert!(!digit.is_match(""));
        assert!(digit.is_match("0"));
        assert!(digit.is_match("1"));
        assert!(!digit.is_match("2"));
        assert!(!digit.is_match("01"));
        assert!(!digit.is_match("00"));

        let mut zeroes = star(byte('0'));
        assert!(zeroes.is_match(""));
        assert!(zeroes.is_match("0"));
        assert!(zeroes.is_match("00"));
        assert!(!zeroes.is_match("1"));
        assert!(!zeroes.is_match("01"));
        assert!(!zeroes.is_match("0010"));

        let mut oh_one = seq(byte('0'), byte('1'));
        assert!(oh_one.is_match("01"));

        let mut integer = alt(byte('0'), seq(byte('1'), star(byte_range('0', '1'))));
        assert!(integer.is_match("0"));
        assert!(!integer.is_match("2"));
        assert!(integer.is_match("10"));
        assert!(!integer.is_match("01"));
        assert!(integer.is_match("1101001"));
        assert!(!integer.is_match("0101001"));
        assert!(!integer.is_match("1101021"));
    }

    // ~4 ns / byte parsed
    #[bench]
    fn this_crate(bencher: &mut Bencher) {
        use combinators::*;

        let integer = alt(
            byte('0'),
            seq(byte_range('1', '9'), star(byte_range('0', '9'))),
        );
        let tail = seq(byte('.'), star(byte_range('0', '9')));
        let mut decimal = seq(integer, maybe(tail));

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
        let number = Regex::new("^(0|[1-9][0-9]*)(\\.[0-9]*)?$").unwrap();
        bencher.iter(|| {
            assert!(number.is_match(ANUM));
            assert!(!number.is_match(NOTANUM));
        })
    }
}
