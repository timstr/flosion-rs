use hashstash::{stash_clone_with_context, Stash};

use crate::{
    core::{
        sound::{
            argument::ArgumentScope,
            soundgraph::SoundGraph,
            soundinput::{BasicProcessorInput, InputOptions},
            soundprocessor::{SoundProcessor, SoundProcessorWithId},
        },
        stashing::{StashingContext, UnstashingContext},
    },
    ui_core::{arguments::ParsedArguments, factories::Factories},
};

use super::testobjects::{TestDynamicSoundProcessor, TestStaticSoundProcessor};

fn test_sound_object_factories() -> Factories {
    let mut factories = Factories::new_empty();

    factories
        .sound_objects_mut()
        .register::<SoundProcessorWithId<TestStaticSoundProcessor>>();
    factories
        .sound_objects_mut()
        .register::<SoundProcessorWithId<TestDynamicSoundProcessor>>();

    factories
}

#[test]
fn stash_clone_basic_input() {
    let input = BasicProcessorInput::new(InputOptions::Synchronous, 2, ArgumentScope::new_empty());

    let stash = Stash::new();
    let factories = test_sound_object_factories();

    let (new_input, _) = stash_clone_with_context(
        &input,
        &stash,
        StashingContext::new_stashing_normally(),
        UnstashingContext::new(factories.sound_objects(), factories.expression_objects()),
    )
    .unwrap();

    assert_eq!(input, new_input);
}

#[test]
fn stash_clone_test_static_processor() {
    let mut proc = TestStaticSoundProcessor::new(&ParsedArguments::new_empty());
    proc.inputs.push(BasicProcessorInput::new(
        InputOptions::Synchronous,
        2,
        ArgumentScope::new_empty(),
    ));

    // ----------------------------------

    let stash = Stash::new();
    let factories = test_sound_object_factories();

    let stash_handle = stash.stash_with_context(&proc, StashingContext::new_stashing_normally());

    // ----------------------------------

    let mut new_proc = TestStaticSoundProcessor::new(&ParsedArguments::new_empty());

    assert_ne!(new_proc.inputs, proc.inputs);

    // ----------------------------------

    stash
        .unstash_inplace_with_context(
            &stash_handle,
            &mut new_proc,
            UnstashingContext::new(factories.sound_objects(), factories.expression_objects()),
        )
        .unwrap();

    assert_eq!(new_proc.inputs, proc.inputs);
}

#[test]
fn stash_clone_empty_graph() {
    let graph = SoundGraph::new();

    let stash = Stash::new();
    let factories = test_sound_object_factories();

    let (new_graph, _) = stash_clone_with_context(
        &graph,
        &stash,
        StashingContext::new_stashing_normally(),
        UnstashingContext::new(factories.sound_objects(), factories.expression_objects()),
    )
    .unwrap();

    assert!(new_graph.sound_processors().is_empty());
}

#[test]
fn stash_clone_graph_with_one_static_processor() {
    let mut graph = SoundGraph::new();

    let mut proc = SoundProcessorWithId::<TestStaticSoundProcessor>::new_default();
    proc.inputs.push(BasicProcessorInput::new(
        InputOptions::Synchronous,
        2,
        ArgumentScope::new_empty(),
    ));
    let proc_id = proc.id();

    graph.add_sound_processor(Box::new(proc));

    let proc = graph
        .sound_processor(proc_id)
        .unwrap()
        .downcast::<TestStaticSoundProcessor>()
        .unwrap();

    // ----------------------------------

    let stash = Stash::new();
    let factories = test_sound_object_factories();

    let (new_graph, _) = stash_clone_with_context(
        &graph,
        &stash,
        StashingContext::new_stashing_normally(),
        UnstashingContext::new(factories.sound_objects(), factories.expression_objects()),
    )
    .unwrap();

    // ----------------------------------

    assert_eq!(new_graph.sound_processors().len(), 1);

    let new_proc = new_graph.sound_processor(proc_id).unwrap();

    let new_proc = new_proc.downcast::<TestStaticSoundProcessor>().unwrap();

    assert_eq!(new_proc.id(), proc.id());
    assert_eq!(new_proc.inputs, proc.inputs);
}

#[test]
fn stash_clone_graph_with_one_dynamic_processor() {
    let mut graph = SoundGraph::new();

    let mut proc = SoundProcessorWithId::<TestDynamicSoundProcessor>::new_default();
    proc.inputs.push(BasicProcessorInput::new(
        InputOptions::Synchronous,
        2,
        ArgumentScope::new_empty(),
    ));
    let proc_id = proc.id();

    graph.add_sound_processor(Box::new(proc));

    let proc = graph
        .sound_processor(proc_id)
        .unwrap()
        .downcast::<TestDynamicSoundProcessor>()
        .unwrap();

    // ----------------------------------

    let stash = Stash::new();
    let factories = test_sound_object_factories();

    let (new_graph, _) = stash_clone_with_context(
        &graph,
        &stash,
        StashingContext::new_stashing_normally(),
        UnstashingContext::new(factories.sound_objects(), factories.expression_objects()),
    )
    .unwrap();

    // ----------------------------------

    assert_eq!(new_graph.sound_processors().len(), 1);

    let new_proc = new_graph.sound_processor(proc_id).unwrap();

    let new_proc = new_proc.downcast::<TestDynamicSoundProcessor>().unwrap();

    assert_eq!(new_proc.id(), proc.id());
    assert_eq!(new_proc.inputs, proc.inputs);
}
