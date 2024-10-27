use hashstash::{stash_clone, stash_clone_with_context, Stash};

use crate::{
    core::{
        expression::expressionobject::ExpressionObjectFactory,
        sound::{
            soundgraph::SoundGraph,
            soundinput::{BasicProcessorInput, InputOptions},
            soundobject::SoundObjectFactory,
            soundprocessor::{SoundProcessor, SoundProcessorWithId},
        },
        stashing::StashingContext,
    },
    ui_core::arguments::ParsedArguments,
};

use super::testobjects::{TestDynamicSoundProcessor, TestStaticSoundProcessor};

fn test_sound_object_factory() -> SoundObjectFactory {
    let mut factory = SoundObjectFactory::new_empty();
    factory.register::<SoundProcessorWithId<TestStaticSoundProcessor>>();
    factory.register::<SoundProcessorWithId<TestDynamicSoundProcessor>>();
    factory
}

fn test_expression_object_factory() -> ExpressionObjectFactory {
    ExpressionObjectFactory::new_empty()
}

#[test]
fn stash_clone_basic_input() {
    let input = BasicProcessorInput::new(InputOptions::Synchronous, 2);

    let stash = Stash::new();

    let (new_input, _) =
        stash_clone_with_context(&input, &stash, &StashingContext::new_stashing_normally())
            .unwrap();

    assert_eq!(input, new_input);
}

#[test]
fn stash_clone_test_static_processor() {
    let mut proc = TestStaticSoundProcessor::new(&ParsedArguments::new_empty());
    proc.inputs
        .push(BasicProcessorInput::new(InputOptions::Synchronous, 2));

    // ----------------------------------

    let stash = Stash::new();

    let stash_handle = stash.stash_with_context(&proc, &StashingContext::new_stashing_normally());

    // ----------------------------------

    let mut new_proc = TestStaticSoundProcessor::new(&ParsedArguments::new_empty());

    assert_ne!(new_proc.inputs, proc.inputs);

    // ----------------------------------

    stash.unstash_inplace(&stash_handle, &mut new_proc).unwrap();

    assert_eq!(new_proc.inputs, proc.inputs);
}

#[test]
fn stash_clone_empty_graph() {
    let graph = SoundGraph::new();

    let stash = Stash::new();
    let sound_object_factory = test_sound_object_factory();
    let expr_object_factory = test_expression_object_factory();

    let (new_graph, _) = graph
        .stash_clone(&stash, &sound_object_factory, &expr_object_factory)
        .unwrap();

    assert!(new_graph.sound_processors().is_empty());
}

#[test]
fn stash_clone_graph_with_one_static_processor() {
    let mut graph = SoundGraph::new();

    let mut proc = SoundProcessorWithId::<TestStaticSoundProcessor>::new_default();
    proc.inputs
        .push(BasicProcessorInput::new(InputOptions::Synchronous, 2));
    let proc_id = proc.id();

    graph.add_sound_processor(Box::new(proc));

    let proc = graph
        .sound_processor(proc_id)
        .unwrap()
        .downcast::<TestStaticSoundProcessor>()
        .unwrap();

    // ----------------------------------

    let stash = Stash::new();
    let sound_object_factory = test_sound_object_factory();
    let expr_obj_factory = test_expression_object_factory();

    let (new_graph, _) = graph
        .stash_clone(&stash, &sound_object_factory, &expr_obj_factory)
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
    proc.inputs
        .push(BasicProcessorInput::new(InputOptions::Synchronous, 2));
    let proc_id = proc.id();

    graph.add_sound_processor(Box::new(proc));

    let proc = graph
        .sound_processor(proc_id)
        .unwrap()
        .downcast::<TestDynamicSoundProcessor>()
        .unwrap();

    // ----------------------------------

    let stash = Stash::new();
    let sound_object_factory = test_sound_object_factory();
    let expr_object_factory = test_expression_object_factory();

    let (new_graph, _) = graph
        .stash_clone(&stash, &sound_object_factory, &expr_object_factory)
        .unwrap();

    // ----------------------------------

    assert_eq!(new_graph.sound_processors().len(), 1);

    let new_proc = new_graph.sound_processor(proc_id).unwrap();

    let new_proc = new_proc.downcast::<TestDynamicSoundProcessor>().unwrap();

    assert_eq!(new_proc.id(), proc.id());
    assert_eq!(new_proc.inputs, proc.inputs);
}
