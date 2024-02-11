use super::Argument;

pub struct FilePathArgument(pub &'static str);

impl Argument for FilePathArgument {
    type ValueType = std::path::PathBuf;

    fn name(&self) -> &'static str {
        self.0
    }

    fn suggestions(s: &str) -> Vec<Self::ValueType> {
        todo!()
        // TODO:
        // - expand '~' to ${HOME} or whatever
        // - glob the full path with a '*' on the end,
        //   list any matches
        // - yo bro what if the suggestions were listed
        //   in a secondary summon widget?
    }

    fn try_parse(s: &str) -> Option<Self::ValueType> {
        Some(s.into())
    }
}
