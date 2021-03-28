# regex-impls

Regex implementations in Rust, for fun.

- `deriv_regex` uses regular expression derivatives and smart constructors, as described in
["Smart constructors are smarter than you think"](http://www.weaselhat.com/2020/05/07/smart-constructors-are-smarter-than-you-think/).
- `combinator_regex` defined a combinator interface for regexes. The interface has the regex expose
  a state, which you can advance by a character, ask whether it accepts at this moment, and such.

For each, I implemented the regex `^(0|[1-9][0-9]*)(\\.[0-9]*)?$`, tested it on a couple length 50
strings, and compared to Burnt Sushi's `regex` crate, which is probably as fast as you can get. The
timing on my desktop is:

                *-------------*------------------*-------------*
                | regex crate | combinator_regex | deriv_regex |
    *-----------*-------------*------------------*-------------*
    | time      | 125 ns      | 430 ns           | 2600 ns     |
    *-----------*-------------*------------------*-------------*
    | time/char | 1.3 ns      | 4.3 ns           | 26 ns       |
    *-----------*-------------*------------------*-------------*
    | memory    | 4000 bytes  | 24 bytes         | grows :-/   |
    *-----------*-------------*------------------*-------------*
