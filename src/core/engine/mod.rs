pub mod compiledexpression;
pub(crate) mod garbage;
pub mod soundgraphcompiler;
pub(crate) mod scratcharena;
pub mod soundengine;
pub mod compiledsoundinput;
pub(crate) mod stategraph;
pub(crate) mod stategraphedit;
pub(crate) mod stategraphnode;
pub(crate) mod stategraphvalidation;

// TODO: consider dropping 'node' from names in this module and
// preferring to use 'compiled', as in 'CompiledSoundProcessor'
// and 'CompiledSoundInput', etc
