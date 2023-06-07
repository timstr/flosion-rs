pub mod context;
pub mod graphobject;
// pub mod graphserialization;
mod path;
mod soundengine;
pub mod soundgraph;
pub(crate) mod soundgraphdata;
pub mod soundgraphedit;
pub mod soundgrapherror;
pub(crate) mod soundgraphtopology;
pub(crate) mod soundgraphvalidation;
pub mod soundinput;
pub mod soundinputnode;
pub mod soundinputtypes;
pub mod soundnumberinput;
pub mod soundnumberinputnode;
pub mod soundnumbersource;
pub mod soundprocessor;
pub mod soundprocessortools;
pub mod state;
mod stategraph;
mod stategraphnode;
mod stategraphvalidation;

#[cfg(test)]
mod test;
