//! The one shape both guards report problems in: `<where>: [<check>] <message>`.

use std::fmt;

use crate::Outcome;

/// One rule broken, at one place.
#[derive(Debug)]
pub struct Problem {
    /// A path, `path:line`, or a dependency edge `from -> to`.
    pub at: String,
    /// The name of the check that failed.
    pub check: &'static str,
    pub message: String,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: [{}] {}", self.at, self.check, self.message)
    }
}

/// The problems a guard found, printed together when it finishes.
#[derive(Debug)]
pub struct Report {
    guard: &'static str,
    problems: Vec<Problem>,
}

impl Report {
    pub fn new(guard: &'static str) -> Self {
        Self {
            guard,
            problems: Vec::new(),
        }
    }

    pub fn add(&mut self, at: impl Into<String>, check: &'static str, message: impl Into<String>) {
        self.problems.push(Problem {
            at: at.into(),
            check,
            message: message.into(),
        });
    }

    pub fn push(&mut self, problem: Problem) {
        self.problems.push(problem);
    }

    /// Prints the problems, or `clean` if there are none, and returns the outcome.
    /// `fix` tells the reader what to do about a failure.
    pub fn finish(self, clean: &str, fix: &str) -> Outcome {
        if self.problems.is_empty() {
            println!("{}: {clean}", self.guard);
            return Outcome::Clean;
        }
        for problem in &self.problems {
            println!("{problem}");
        }
        println!("{}: {} problem(s). {fix}", self.guard, self.problems.len());
        Outcome::Violations
    }
}

/// `path:line`, with lines numbered from 1.
pub fn at_line(path: &str, index: usize) -> String {
    format!("{path}:{}", index + 1)
}

/// The lines of a byte buffer, split on `\n`.
pub fn lines(bytes: &[u8]) -> impl Iterator<Item = (usize, &[u8])> {
    bytes.split(|&b| b == b'\n').enumerate()
}
