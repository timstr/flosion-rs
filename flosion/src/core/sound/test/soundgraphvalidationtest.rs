use crate::core::sound::{
    sounderror::SoundError,
    soundgraph::SoundGraph,
    soundgraphvalidation::find_sound_error,
    soundinput::{BasicProcessorInput, InputOptions},
    soundprocessor::SoundProcessorWithId,
    test::testobjects::{TestDynamicSoundProcessor, TestStaticSoundProcessor},
};

#[test]
fn find_error_empty_graph() {
    let graph = SoundGraph::new();
    assert_eq!(find_sound_error(&graph), None);
}

#[test]
fn find_error_one_static_proc() {
    let proc = SoundProcessorWithId::<TestStaticSoundProcessor>::new_default();

    let mut graph = SoundGraph::new();

    graph.add_sound_processor(Box::new(proc));

    assert_eq!(find_sound_error(&graph), None);
}

#[test]
fn find_error_one_dynamic_proc() {
    let proc = SoundProcessorWithId::<TestDynamicSoundProcessor>::new_default();

    let mut graph = SoundGraph::new();

    graph.add_sound_processor(Box::new(proc));

    assert_eq!(find_sound_error(&graph), None);
}

#[test]
fn find_error_static_to_self_cycle() {
    let mut proc = SoundProcessorWithId::<TestStaticSoundProcessor>::new_default();

    proc.inputs
        .push(BasicProcessorInput::new(InputOptions::Synchronous, 1));

    let proc_id = proc.id();

    proc.inputs[0].set_target(Some(proc_id));

    let mut graph = SoundGraph::new();
    graph.add_sound_processor(Box::new(proc));

    assert_eq!(
        find_sound_error(&graph),
        Some(SoundError::CircularDependency),
    );
}

#[test]
fn find_error_two_static_procs_singly_connected() {
    let proc1 = SoundProcessorWithId::<TestStaticSoundProcessor>::new_default();

    let mut proc2 = SoundProcessorWithId::<TestStaticSoundProcessor>::new_default();

    proc2
        .inputs
        .push(BasicProcessorInput::new(InputOptions::Synchronous, 1));

    proc2.inputs[0].set_target(Some(proc1.id()));

    let mut graph = SoundGraph::new();
    graph.add_sound_processor(Box::new(proc1));
    graph.add_sound_processor(Box::new(proc2));

    assert_eq!(find_sound_error(&graph), None,);
}

#[test]
fn find_error_two_static_procs_doubly_connected() {
    let proc1 = SoundProcessorWithId::<TestStaticSoundProcessor>::new_default();

    let mut proc2 = SoundProcessorWithId::<TestStaticSoundProcessor>::new_default();

    proc2
        .inputs
        .push(BasicProcessorInput::new(InputOptions::Synchronous, 1));
    proc2
        .inputs
        .push(BasicProcessorInput::new(InputOptions::Synchronous, 1));

    proc2.inputs[0].set_target(Some(proc1.id()));
    proc2.inputs[1].set_target(Some(proc1.id()));

    let mut graph = SoundGraph::new();
    graph.add_sound_processor(Box::new(proc1));
    graph.add_sound_processor(Box::new(proc2));

    assert_eq!(find_sound_error(&graph), None,);
}

#[test]
fn find_error_static_to_dynamic_no_branches() {
    let mut static_proc = SoundProcessorWithId::<TestStaticSoundProcessor>::new_default();

    let dynamic_proc = SoundProcessorWithId::<TestDynamicSoundProcessor>::new_default();

    static_proc
        .inputs
        .push(BasicProcessorInput::new(InputOptions::Synchronous, 0));
    static_proc.inputs[0].set_target(Some(dynamic_proc.id()));

    let mut graph = SoundGraph::new();
    graph.add_sound_processor(Box::new(static_proc));
    graph.add_sound_processor(Box::new(dynamic_proc));

    assert_eq!(find_sound_error(&graph), None);
}

#[test]
fn find_error_static_to_dynamic_one_branch() {
    let mut static_proc = SoundProcessorWithId::<TestStaticSoundProcessor>::new_default();

    let dynamic_proc = SoundProcessorWithId::<TestDynamicSoundProcessor>::new_default();

    static_proc
        .inputs
        .push(BasicProcessorInput::new(InputOptions::Synchronous, 1));
    static_proc.inputs[0].set_target(Some(dynamic_proc.id()));

    let mut graph = SoundGraph::new();
    graph.add_sound_processor(Box::new(static_proc));
    graph.add_sound_processor(Box::new(dynamic_proc));

    assert_eq!(find_sound_error(&graph), None);
}

#[test]
fn find_error_static_to_dynamic_two_branches() {
    let mut static_proc = SoundProcessorWithId::<TestStaticSoundProcessor>::new_default();

    let dynamic_proc = SoundProcessorWithId::<TestDynamicSoundProcessor>::new_default();

    static_proc
        .inputs
        .push(BasicProcessorInput::new(InputOptions::Synchronous, 2));
    static_proc.inputs[0].set_target(Some(dynamic_proc.id()));

    let mut graph = SoundGraph::new();
    graph.add_sound_processor(Box::new(static_proc));
    graph.add_sound_processor(Box::new(dynamic_proc));

    assert_eq!(find_sound_error(&graph), None);
}

#[test]
fn find_error_static_to_static_no_branches() {
    let proc1 = SoundProcessorWithId::<TestStaticSoundProcessor>::new_default();
    let proc1_id = proc1.id();

    let mut proc2 = SoundProcessorWithId::<TestStaticSoundProcessor>::new_default();

    proc2
        .inputs
        .push(BasicProcessorInput::new(InputOptions::Synchronous, 0));
    proc2.inputs[0].set_target(Some(proc1.id()));

    let mut graph = SoundGraph::new();
    graph.add_sound_processor(Box::new(proc1));
    graph.add_sound_processor(Box::new(proc2));

    assert_eq!(
        find_sound_error(&graph),
        Some(SoundError::StaticNotOneState(proc1_id))
    );
}

#[test]
fn find_error_static_to_static_one_branch() {
    let proc1 = SoundProcessorWithId::<TestStaticSoundProcessor>::new_default();

    let mut proc2 = SoundProcessorWithId::<TestStaticSoundProcessor>::new_default();

    proc2
        .inputs
        .push(BasicProcessorInput::new(InputOptions::Synchronous, 1));
    proc2.inputs[0].set_target(Some(proc1.id()));

    let mut graph = SoundGraph::new();
    graph.add_sound_processor(Box::new(proc1));
    graph.add_sound_processor(Box::new(proc2));

    assert_eq!(find_sound_error(&graph), None);
}

#[test]
fn find_error_static_to_static_two_branches() {
    let proc1 = SoundProcessorWithId::<TestStaticSoundProcessor>::new_default();
    let proc1_id = proc1.id();

    let mut proc2 = SoundProcessorWithId::<TestStaticSoundProcessor>::new_default();

    proc2
        .inputs
        .push(BasicProcessorInput::new(InputOptions::Synchronous, 2));
    proc2.inputs[0].set_target(Some(proc1.id()));

    let mut graph = SoundGraph::new();
    graph.add_sound_processor(Box::new(proc1));
    graph.add_sound_processor(Box::new(proc2));

    assert_eq!(
        find_sound_error(&graph),
        Some(SoundError::StaticNotOneState(proc1_id))
    );
}

#[test]
fn find_error_static_to_dynamic_one_branch_nonsync() {
    let mut static_proc = SoundProcessorWithId::<TestStaticSoundProcessor>::new_default();
    let dynamic_proc = SoundProcessorWithId::<TestDynamicSoundProcessor>::new_default();

    static_proc
        .inputs
        .push(BasicProcessorInput::new(InputOptions::NonSynchronous, 1));
    static_proc.inputs[0].set_target(Some(dynamic_proc.id()));

    let mut graph = SoundGraph::new();
    graph.add_sound_processor(Box::new(static_proc));
    graph.add_sound_processor(Box::new(dynamic_proc));

    assert_eq!(find_sound_error(&graph), None);
}

#[test]
fn find_error_static_to_static_one_branch_nonsync() {
    let mut static_proc = SoundProcessorWithId::<TestStaticSoundProcessor>::new_default();
    let other_static_proc = SoundProcessorWithId::<TestStaticSoundProcessor>::new_default();
    let other_proc_id = other_static_proc.id();

    static_proc
        .inputs
        .push(BasicProcessorInput::new(InputOptions::NonSynchronous, 1));
    static_proc.inputs[0].set_target(Some(other_static_proc.id()));

    let mut graph = SoundGraph::new();
    graph.add_sound_processor(Box::new(static_proc));
    graph.add_sound_processor(Box::new(other_static_proc));

    assert_eq!(
        find_sound_error(&graph),
        Some(SoundError::StaticNotSynchronous(other_proc_id))
    );
}

#[test]
fn find_error_dynamic_to_static_no_branches() {
    let static_proc = SoundProcessorWithId::<TestStaticSoundProcessor>::new_default();
    let static_proc_id = static_proc.id();
    let mut dynamic_proc = SoundProcessorWithId::<TestDynamicSoundProcessor>::new_default();

    dynamic_proc
        .inputs
        .push(BasicProcessorInput::new(InputOptions::Synchronous, 0));
    dynamic_proc.inputs[0].set_target(Some(static_proc.id()));

    let mut graph = SoundGraph::new();
    graph.add_sound_processor(Box::new(static_proc));
    graph.add_sound_processor(Box::new(dynamic_proc));

    assert_eq!(
        find_sound_error(&graph),
        Some(SoundError::StaticNotOneState(static_proc_id))
    );
}

#[test]
fn find_error_dynamic_to_static_one_branch() {
    let static_proc = SoundProcessorWithId::<TestStaticSoundProcessor>::new_default();
    let mut dynamic_proc = SoundProcessorWithId::<TestDynamicSoundProcessor>::new_default();

    dynamic_proc
        .inputs
        .push(BasicProcessorInput::new(InputOptions::Synchronous, 1));
    dynamic_proc.inputs[0].set_target(Some(static_proc.id()));

    let mut graph = SoundGraph::new();
    graph.add_sound_processor(Box::new(static_proc));
    graph.add_sound_processor(Box::new(dynamic_proc));

    assert_eq!(find_sound_error(&graph), None);
}

#[test]
fn find_error_dynamic_to_static_two_branches() {
    let static_proc = SoundProcessorWithId::<TestStaticSoundProcessor>::new_default();
    let static_proc_id = static_proc.id();
    let mut dynamic_proc = SoundProcessorWithId::<TestDynamicSoundProcessor>::new_default();

    dynamic_proc
        .inputs
        .push(BasicProcessorInput::new(InputOptions::Synchronous, 2));
    dynamic_proc.inputs[0].set_target(Some(static_proc.id()));

    let mut graph = SoundGraph::new();
    graph.add_sound_processor(Box::new(static_proc));
    graph.add_sound_processor(Box::new(dynamic_proc));

    assert_eq!(
        find_sound_error(&graph),
        Some(SoundError::StaticNotOneState(static_proc_id))
    );
}

#[test]
fn find_error_dynamic_to_static_nonsync() {
    let static_proc = SoundProcessorWithId::<TestStaticSoundProcessor>::new_default();
    let static_proc_id = static_proc.id();
    let mut dynamic_proc = SoundProcessorWithId::<TestDynamicSoundProcessor>::new_default();

    dynamic_proc
        .inputs
        .push(BasicProcessorInput::new(InputOptions::NonSynchronous, 1));
    dynamic_proc.inputs[0].set_target(Some(static_proc.id()));

    let mut graph = SoundGraph::new();
    graph.add_sound_processor(Box::new(static_proc));
    graph.add_sound_processor(Box::new(dynamic_proc));

    assert_eq!(
        find_sound_error(&graph),
        Some(SoundError::StaticNotSynchronous(static_proc_id))
    );
}

#[test]
fn find_error_dynamic_to_dynamic_to_static_no_branches() {
    let mut proc1 = SoundProcessorWithId::<TestDynamicSoundProcessor>::new_default();
    let mut proc2 = SoundProcessorWithId::<TestDynamicSoundProcessor>::new_default();
    let proc3 = SoundProcessorWithId::<TestStaticSoundProcessor>::new_default();
    let proc3_id = proc3.id();

    proc1
        .inputs
        .push(BasicProcessorInput::new(InputOptions::Synchronous, 0));
    proc1.inputs[0].set_target(Some(proc2.id()));

    proc2
        .inputs
        .push(BasicProcessorInput::new(InputOptions::Synchronous, 1));
    proc2.inputs[0].set_target(Some(proc3.id()));

    let mut graph = SoundGraph::new();
    graph.add_sound_processor(Box::new(proc1));
    graph.add_sound_processor(Box::new(proc2));
    graph.add_sound_processor(Box::new(proc3));

    assert_eq!(
        find_sound_error(&graph),
        Some(SoundError::StaticNotOneState(proc3_id))
    );
}

#[test]
fn find_error_dynamic_to_dynamic_to_static_one_branch() {
    let mut proc1 = SoundProcessorWithId::<TestDynamicSoundProcessor>::new_default();
    let mut proc2 = SoundProcessorWithId::<TestDynamicSoundProcessor>::new_default();
    let proc3 = SoundProcessorWithId::<TestStaticSoundProcessor>::new_default();

    proc1
        .inputs
        .push(BasicProcessorInput::new(InputOptions::Synchronous, 1));
    proc1.inputs[0].set_target(Some(proc2.id()));

    proc2
        .inputs
        .push(BasicProcessorInput::new(InputOptions::Synchronous, 1));
    proc2.inputs[0].set_target(Some(proc3.id()));

    let mut graph = SoundGraph::new();
    graph.add_sound_processor(Box::new(proc1));
    graph.add_sound_processor(Box::new(proc2));
    graph.add_sound_processor(Box::new(proc3));

    assert_eq!(find_sound_error(&graph), None);
}

#[test]
fn find_error_dynamic_to_dynamic_to_static_cycle() {
    let mut proc1 = SoundProcessorWithId::<TestDynamicSoundProcessor>::new_default();
    let mut proc2 = SoundProcessorWithId::<TestDynamicSoundProcessor>::new_default();
    let mut proc3 = SoundProcessorWithId::<TestStaticSoundProcessor>::new_default();

    proc1
        .inputs
        .push(BasicProcessorInput::new(InputOptions::Synchronous, 0));
    proc1.inputs[0].set_target(Some(proc2.id()));

    proc2
        .inputs
        .push(BasicProcessorInput::new(InputOptions::Synchronous, 1));
    proc2.inputs[0].set_target(Some(proc3.id()));

    proc3
        .inputs
        .push(BasicProcessorInput::new(InputOptions::Synchronous, 1));
    proc3.inputs[0].set_target(Some(proc1.id()));

    let mut graph = SoundGraph::new();
    graph.add_sound_processor(Box::new(proc1));
    graph.add_sound_processor(Box::new(proc2));
    graph.add_sound_processor(Box::new(proc3));

    assert_eq!(
        find_sound_error(&graph),
        Some(SoundError::CircularDependency)
    );
}

#[test]
fn find_error_dynamic_to_dynamic_to_static_two_branches() {
    let mut proc1 = SoundProcessorWithId::<TestDynamicSoundProcessor>::new_default();
    let mut proc2 = SoundProcessorWithId::<TestDynamicSoundProcessor>::new_default();
    let proc3 = SoundProcessorWithId::<TestStaticSoundProcessor>::new_default();
    let proc3_id = proc3.id();

    proc1
        .inputs
        .push(BasicProcessorInput::new(InputOptions::Synchronous, 2));
    proc1.inputs[0].set_target(Some(proc2.id()));

    proc2
        .inputs
        .push(BasicProcessorInput::new(InputOptions::Synchronous, 1));
    proc2.inputs[0].set_target(Some(proc3.id()));

    let mut graph = SoundGraph::new();
    graph.add_sound_processor(Box::new(proc1));
    graph.add_sound_processor(Box::new(proc2));
    graph.add_sound_processor(Box::new(proc3));

    assert_eq!(
        find_sound_error(&graph),
        Some(SoundError::StaticNotOneState(proc3_id))
    );
}

#[test]
fn find_error_dynamic_to_dynamic_to_static_nonsync() {
    let mut proc1 = SoundProcessorWithId::<TestDynamicSoundProcessor>::new_default();
    let mut proc2 = SoundProcessorWithId::<TestDynamicSoundProcessor>::new_default();
    let proc3 = SoundProcessorWithId::<TestStaticSoundProcessor>::new_default();
    let proc3_id = proc3.id();

    proc1
        .inputs
        .push(BasicProcessorInput::new(InputOptions::NonSynchronous, 1));
    proc1.inputs[0].set_target(Some(proc2.id()));

    proc2
        .inputs
        .push(BasicProcessorInput::new(InputOptions::Synchronous, 1));
    proc2.inputs[0].set_target(Some(proc3.id()));

    let mut graph = SoundGraph::new();
    graph.add_sound_processor(Box::new(proc1));
    graph.add_sound_processor(Box::new(proc2));
    graph.add_sound_processor(Box::new(proc3));

    assert_eq!(
        find_sound_error(&graph),
        Some(SoundError::StaticNotSynchronous(proc3_id))
    );
}

#[test]
fn find_error_dynamic_indirect_fork_to_static() {
    let mut proc_root1 = SoundProcessorWithId::<TestDynamicSoundProcessor>::new_default();
    let mut proc_root2 = SoundProcessorWithId::<TestDynamicSoundProcessor>::new_default();
    let mut proc_middle = SoundProcessorWithId::<TestDynamicSoundProcessor>::new_default();
    let proc_leaf = SoundProcessorWithId::<TestStaticSoundProcessor>::new_default();
    let proc_leaf_id = proc_leaf.id();

    proc_root1
        .inputs
        .push(BasicProcessorInput::new(InputOptions::Synchronous, 1));
    proc_root1.inputs[0].set_target(Some(proc_middle.id()));

    proc_root2
        .inputs
        .push(BasicProcessorInput::new(InputOptions::Synchronous, 1));
    proc_root2.inputs[0].set_target(Some(proc_middle.id()));

    proc_middle
        .inputs
        .push(BasicProcessorInput::new(InputOptions::Synchronous, 1));
    proc_middle.inputs[0].set_target(Some(proc_leaf.id()));

    let mut graph = SoundGraph::new();
    graph.add_sound_processor(Box::new(proc_root1));
    graph.add_sound_processor(Box::new(proc_root2));
    graph.add_sound_processor(Box::new(proc_middle));
    graph.add_sound_processor(Box::new(proc_leaf));

    assert_eq!(
        find_sound_error(&graph),
        Some(SoundError::StaticNotOneState(proc_leaf_id))
    );
}

#[test]
fn find_error_dynamic_direct_fork_to_static_nonsync() {
    let mut proc_root1 = SoundProcessorWithId::<TestDynamicSoundProcessor>::new_default();
    let mut proc_root2 = SoundProcessorWithId::<TestDynamicSoundProcessor>::new_default();
    let proc_leaf = SoundProcessorWithId::<TestStaticSoundProcessor>::new_default();

    proc_root1
        .inputs
        .push(BasicProcessorInput::new(InputOptions::Synchronous, 1));
    proc_root1.inputs[0].set_target(Some(proc_leaf.id()));

    proc_root2
        .inputs
        .push(BasicProcessorInput::new(InputOptions::Synchronous, 1));
    proc_root2.inputs[0].set_target(Some(proc_leaf.id()));

    let mut graph = SoundGraph::new();
    graph.add_sound_processor(Box::new(proc_root1));
    graph.add_sound_processor(Box::new(proc_root2));
    graph.add_sound_processor(Box::new(proc_leaf));

    assert_eq!(find_sound_error(&graph), None);
}

#[test]
fn find_error_dynamic_to_static_two_inputs() {
    let mut dynamic_proc = SoundProcessorWithId::<TestDynamicSoundProcessor>::new_default();
    let static_proc = SoundProcessorWithId::<TestStaticSoundProcessor>::new_default();

    dynamic_proc
        .inputs
        .push(BasicProcessorInput::new(InputOptions::Synchronous, 1));
    dynamic_proc
        .inputs
        .push(BasicProcessorInput::new(InputOptions::Synchronous, 1));
    dynamic_proc.inputs[0].set_target(Some(static_proc.id()));
    dynamic_proc.inputs[1].set_target(Some(static_proc.id()));

    let mut graph = SoundGraph::new();
    graph.add_sound_processor(Box::new(dynamic_proc));
    graph.add_sound_processor(Box::new(static_proc));

    assert_eq!(find_sound_error(&graph), None);
}

#[test]
fn find_error_dynamic_to_static_two_inputs_with_side_proc() {
    let mut dynamic_proc = SoundProcessorWithId::<TestDynamicSoundProcessor>::new_default();
    let mut side_dynamic_proc = SoundProcessorWithId::<TestDynamicSoundProcessor>::new_default();
    let static_proc = SoundProcessorWithId::<TestStaticSoundProcessor>::new_default();

    dynamic_proc
        .inputs
        .push(BasicProcessorInput::new(InputOptions::Synchronous, 1));
    dynamic_proc
        .inputs
        .push(BasicProcessorInput::new(InputOptions::Synchronous, 1));
    dynamic_proc.inputs[0].set_target(Some(side_dynamic_proc.id()));
    dynamic_proc.inputs[1].set_target(Some(static_proc.id()));

    side_dynamic_proc
        .inputs
        .push(BasicProcessorInput::new(InputOptions::Synchronous, 1));
    side_dynamic_proc.inputs[0].set_target(Some(static_proc.id()));

    let mut graph = SoundGraph::new();
    graph.add_sound_processor(Box::new(dynamic_proc));
    graph.add_sound_processor(Box::new(side_dynamic_proc));
    graph.add_sound_processor(Box::new(static_proc));

    assert_eq!(find_sound_error(&graph), None);
}
