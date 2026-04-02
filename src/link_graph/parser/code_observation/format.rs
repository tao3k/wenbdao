use super::CodeObservation;
use std::fmt;

impl fmt::Display for CodeObservation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, ":OBSERVE: lang:{}", self.language)?;
        if let Some(ref scope) = self.scope {
            write!(f, " scope:\"{}\"", scope.replace('"', "\\\""))?;
        }
        write!(f, " \"{}\"", self.pattern.replace('"', "\\\""))
    }
}
