pub struct Regex(Vec<Chunk>);

#[derive(Clone, Copy)]
struct Chunk {
    nullable: bool,
    data: Data,
}

#[derive(Clone, Copy)]
enum Data {
    Void,
    Epsilon,
    Char(char),
    Seq,
    Alt,
    Star,
}

impl Regex {
    fn derivative(&mut self, c: char, regex: &mut impl Iterator<Item = Chunk>) {
        use Data::*;

        let chunk = regex.next().unwrap();
        match chunk.data {
            Void => self.0.push(Chunk {
                nullable: false,
                data: Void,
            }),
            Alt => {
                self.derivative(c, regex);
                let mut nullable = self.0.last().unwrap().nullable;
                self.derivative(c, regex);
                nullable |= self.0.last().unwrap().nullable;
                self.0.push(Chunk {
                    nullable,
                    data: Alt,
                });
            }
            Seq => {}

            _ => unimplemented!(),
        }
    }
}
