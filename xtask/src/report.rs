//! The one shape both guards report problems in: `<where>: [<check>] <message>`.

use std::fmt;

use crate::Outcome;

/// A guard's named checks. Each guard has its own enum of them.
pub trait Check: Copy + fmt::Debug {
    /// The name shown in reports (and, for the leak scan, used in the allowlist).
    fn name(self) -> &'static str;

    /// A problem this check found at `at`.
    fn problem(self, at: impl Into<String>, message: impl Into<String>) -> Problem<Self> {
        Problem {
            at: at.into(),
            check: self,
            message: message.into(),
        }
    }
}

/// One rule broken, at one place.
#[derive(Debug)]
pub struct Problem<C> {
    /// A path, `path:line`, or a dependency edge `from -> to`.
    pub at: String,
    pub check: C,
    pub message: String,
}

impl<C: Check> fmt::Display for Problem<C> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: [{}] {}", self.at, self.check.name(), self.message)
    }
}

/// The problems a guard found, printed together when it finishes.
#[derive(Debug)]
pub struct Report<C> {
    guard: &'static str,
    problems: Vec<Problem<C>>,
}

impl<C: Check> Report<C> {
    pub fn new(guard: &'static str) -> Self {
        Self {
            guard,
            problems: Vec::new(),
        }
    }

    pub fn push(&mut self, problem: Problem<C>) {
        self.problems.push(problem);
    }

    /// Prints the problems, or `clean_summary` if there are none, and returns the outcome.
    /// `fix_hint` tells the reader what to do about a failure.
    pub fn finish(self, clean_summary: &str, fix_hint: &str) -> Outcome {
        if self.problems.is_empty() {
            println!("{}: {clean_summary}", self.guard);
            return Outcome::Clean;
        }
        for problem in &self.problems {
            println!("{problem}");
        }
        println!(
            "{}: {} problem(s). {fix_hint}",
            self.guard,
            self.problems.len()
        );
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
