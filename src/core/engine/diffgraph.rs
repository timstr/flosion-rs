use crate::core::{
    engine::{stategraph::StateGraph, stategraphvalidation::state_graph_matches_sound_graph},
    jit::cache::JitCache,
    sound::soundgraph::SoundGraph,
};

use super::{
    soundgraphcompiler::SoundGraphCompiler, stategraphedit::StateGraphEdit,
    stategraphnode::StateGraphNodeValue,
};

pub(crate) fn diff_sound_graph<'ctx>(
    graph_before: &SoundGraph,
    graph_after: &SoundGraph,
    jit_cache: &JitCache<'ctx>,
) -> Vec<StateGraphEdit<'ctx>> {
    let mut edits = Vec::new();

    // sound graph and state graph should match
    // TODO: re-enable this check. Consider serializing the graph, sending the
    // binary, and deserializing on the audio thread (performance hit is ok
    // in debug mode)
    // #[cfg(debug_assertions)]
    // {
    //     let graph_clone = graph_before.clone();
    //     edits.push(StateGraphEdit::DebugInspection(Box::new(
    //         |sg: &StateGraph<'ctx>| {
    //             let graph = graph_clone;
    //             debug_assert!(
    //                 state_graph_matches_sound_graph(sg, &graph),
    //                 "State graph failed to match sound graph before any updates were made"
    //             );
    //         },
    //     )));
    // }

    // TODO: diff current and new topology and create a list of fine-grained state graph edits
    // HACK deleting everything and then adding it back
    for proc in graph_before.sound_processors().values() {
        if proc.instance().is_static() {
            edits.push(StateGraphEdit::RemoveStaticSoundProcessor(proc.id()));
        }
    }
    // all should be deleted now
    #[cfg(debug_assertions)]
    {
        edits.push(StateGraphEdit::DebugInspection(Box::new(
            |sg: &StateGraph<'ctx>| {
                debug_assert!(sg.static_processors().is_empty());
            },
        )));
    }

    // Add back static processors with populated inputs
    // Note that SoundGraphCompiler will cache and reuse shared static processor
    // nodes, and so no extra book-keeping is needed here to ensure
    // that static processors are allocated only once and reused.
    let mut compiler = SoundGraphCompiler::new(&graph_after, jit_cache);
    for proc in graph_after.sound_processors().values() {
        if proc.instance().is_static() {
            let StateGraphNodeValue::Shared(node) = compiler.compile_sound_processor(proc.id())
            else {
                panic!("Static sound processors must compile to shared state graph nodes");
            };
            edits.push(StateGraphEdit::AddStaticSoundProcessor(node));
        }
    }

    // topology and state graph should still match
    // TODO: re-enable this check
    // #[cfg(debug_assertions)]
    // {
    //     let graph_clone = graph_after.clone();
    //     edits.push(StateGraphEdit::DebugInspection(Box::new(
    //         |sg: &StateGraph<'ctx>| {
    //             let graph = graph_clone;
    //             debug_assert!(
    //                 state_graph_matches_sound_graph(sg, &graph),
    //                 "State graph no longer matches topology after applying updates"
    //             );
    //         },
    //     )));
    // }

    edits
}