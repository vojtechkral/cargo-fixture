use std::{env, ffi::{OsString, OsStr}};

use nom::{InputTake, bytes::complete::tag, InputLength, Compare, CompareResult, combinator::opt, UnspecializedInput, InputIter};

use crate::logger::LogLevel;

#[derive(Debug)]
pub struct Cli {
    pub log_level: LogLevel,
}

pub fn parse() -> Cli {
    let args: Vec<_> = env::args_os().skip(1).collect();
    dbg!(&args);

    // let mut fixture = opt(tag::<_, Args, nom::error::Error<Args>>(Arg("fixture".as_ref())));
    // let f = [OsString::from("fixture".to_string())];
    let f = OsString::from("fixture".to_string());
    let mut fixture = opt(tag::<_, _, nom::error::Error<Args>>(Arg(&f)));
    dbg!(fixture(Args(&args[..])));

    todo!()
}

#[derive(Clone, Debug)]
struct Arg(OsString);

impl InputLength for Arg {
    fn input_len(&self) -> usize {
        1
    }
}

#[derive(Clone, Copy, Debug)]
struct Args<'a>(&'a [OsString]);

impl<'a> InputIter for Args<'a> {
    type Item = &'a [OsString];
    type Iter;
    type IterElem;

    fn iter_indices(&self) -> Self::Iter {
        todo!()
    }

    fn iter_elements(&self) -> Self::IterElem {
        todo!()
    }

    fn position<P>(&self, predicate: P) -> Option<usize>
      where
        P: Fn(Self::Item) -> bool {
        todo!()
    }

    fn slice_index(&self, count: usize) -> Result<usize, nom::Needed> {
        todo!()
    }
}

impl<'a> UnspecializedInput for Args<'a> {}

impl<'a> InputTake for Args<'a> {
    fn take(&self, count: usize) -> Self {
        Self(&self.0[0..count])
    }

    fn take_split(&self, count: usize) -> (Self, Self) {
        let (a, b) = self.0.split_at(count);
        // WARN: opposite order is expected to be returned:
        (Self(b), Self(a))
    }
}

// impl<'a> Compare<Arg<'a>> for Args<'a> {
//     fn compare(&self, t: Arg<'a>) -> CompareResult {
//         dbg!(self);
//         dbg!(t);
//         if self.0 == &[t.0] {
//             dbg!(CompareResult::Ok)
//         } else {
//             dbg!(CompareResult::Error)
//         }
//     }

//     fn compare_no_case(&self, t: Arg<'a>) -> CompareResult {
//         if let &[one] = &self.0 {
//             if one.eq_ignore_ascii_case(t.0) {
//                 return CompareResult::Ok;
//             }
//         }

//         CompareResult::Error
//     }
// }

// impl<'a> Compare<Args<'a>> for Args<'a> {
//     fn compare(&self, t: Args<'a>) -> CompareResult {
//         if self.0.len() < t.0.len() {
//             CompareResult::Incomplete
//         } else {
//             if self.0.iter().zip(t.0.iter()).all(|(a, b)| a == b) {
//                 CompareResult::Ok
//             } else {
//                 CompareResult::Error
//             }
//         }
//     }

//     fn compare_no_case(&self, t: Args<'a>) -> CompareResult {
//         if self.0.len() < t.0.len() {
//             CompareResult::Incomplete
//         } else {
//             if self.0.iter().zip(t.0.iter()).all(|(a, b)| a.eq_ignore_ascii_case(b)) {
//                 CompareResult::Ok
//             } else {
//                 CompareResult::Error
//             }
//         }
//     }
// }

impl<'a> InputLength for Args<'a> {
    fn input_len(&self) -> usize {
        self.0.len()
    }
}

mod arg {
    use nom::IResult;

    // pub fn tag(tag: &str) -> IResult<Args>
}
