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

// #[test]
// fn find_error_static_to_dynamic_no_branches() {
//     let mut graph = SoundGraph::new();

//     let static_proc = graph
//         .add_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let dynamic_proc = graph
//         .add_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let input_id = graph
//         .add_sound_input(static_proc.id(), InputOptions::Synchronous, vec![])
//         .unwrap();

//     graph
//         .connect_sound_input(input_id, dynamic_proc.id())
//         .unwrap();

//     assert_eq!(find_sound_error(&graph), None);
// }

// #[test]
// fn find_error_static_to_dynamic_one_branch() {
//     let mut graph = SoundGraph::new();

//     let static_proc = graph
//         .add_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let dynamic_proc = graph
//         .add_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let input_id = graph
//         .add_sound_input(
//             static_proc.id(),
//             InputOptions::Synchronous,
//             vec![SoundInputBranchId::new(1)],
//         )
//         .unwrap();

//     graph
//         .connect_sound_input(input_id, dynamic_proc.id())
//         .unwrap();

//     assert_eq!(find_sound_error(&graph), None);
// }

// #[test]
// fn find_error_static_to_dynamic_two_branches() {
//     let mut graph = SoundGraph::new();

//     let static_proc = graph
//         .add_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let dynamic_proc = graph
//         .add_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let input_id = graph
//         .add_sound_input(
//             static_proc.id(),
//             InputOptions::Synchronous,
//             vec![SoundInputBranchId::new(1), SoundInputBranchId::new(2)],
//         )
//         .unwrap();

//     graph
//         .connect_sound_input(input_id, dynamic_proc.id())
//         .unwrap();

//     assert_eq!(find_sound_error(&graph), None);
// }

// #[test]
// fn find_error_static_to_static_no_branches() {
//     let mut graph = SoundGraph::new();

//     let proc1 = graph
//         .add_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let proc2 = graph
//         .add_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let input_id = graph
//         .add_sound_input(proc1.id(), InputOptions::Synchronous, vec![])
//         .unwrap();

//     graph.connect_sound_input(input_id, proc2.id()).unwrap();

//     assert_eq!(
//         find_sound_error(&graph),
//         Some(SoundError::StaticNotOneState(proc2.id()))
//     );
// }

// #[test]
// fn find_error_static_to_static_one_branch() {
//     let mut graph = SoundGraph::new();

//     let proc1 = graph
//         .add_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let proc2 = graph
//         .add_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let input_id = graph
//         .add_sound_input(
//             proc1.id(),
//             InputOptions::Synchronous,
//             vec![SoundInputBranchId::new(1)],
//         )
//         .unwrap();

//     graph.connect_sound_input(input_id, proc2.id()).unwrap();

//     assert_eq!(find_sound_error(&graph), None);
// }

// #[test]
// fn find_error_static_to_static_two_branches() {
//     let mut graph = SoundGraph::new();

//     let proc1 = graph
//         .add_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let proc2 = graph
//         .add_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let input_id = graph
//         .add_sound_input(
//             proc1.id(),
//             InputOptions::Synchronous,
//             vec![SoundInputBranchId::new(1), SoundInputBranchId::new(2)],
//         )
//         .unwrap();

//     graph.connect_sound_input(input_id, proc2.id()).unwrap();

//     assert_eq!(
//         find_sound_error(&graph),
//         Some(SoundError::StaticNotOneState(proc2.id()))
//     );
// }

// #[test]
// fn find_error_static_to_dynamic_one_branch_nonsync() {
//     let mut graph = SoundGraph::new();

//     let proc1 = graph
//         .add_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let proc2 = graph
//         .add_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let input_id = graph
//         .add_sound_input(
//             proc1.id(),
//             InputOptions::NonSynchronous,
//             vec![SoundInputBranchId::new(1)],
//         )
//         .unwrap();

//     graph.connect_sound_input(input_id, proc2.id()).unwrap();

//     assert_eq!(find_sound_error(&graph), None);
// }

// #[test]
// fn find_error_static_to_static_one_branch_nonsync() {
//     let mut graph = SoundGraph::new();

//     let proc1 = graph
//         .add_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let proc2 = graph
//         .add_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let input_id = graph
//         .add_sound_input(
//             proc1.id(),
//             InputOptions::NonSynchronous,
//             vec![SoundInputBranchId::new(1)],
//         )
//         .unwrap();

//     graph.connect_sound_input(input_id, proc2.id()).unwrap();

//     assert_eq!(
//         find_sound_error(&graph),
//         Some(SoundError::StaticNotSynchronous(proc2.id()))
//     );
// }

// #[test]
// fn find_error_dynamic_to_static_no_branches() {
//     let mut graph = SoundGraph::new();

//     let proc1 = graph
//         .add_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let proc2 = graph
//         .add_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let input_id = graph
//         .add_sound_input(proc1.id(), InputOptions::Synchronous, vec![])
//         .unwrap();

//     graph.connect_sound_input(input_id, proc2.id()).unwrap();

//     assert_eq!(
//         find_sound_error(&graph),
//         Some(SoundError::StaticNotOneState(proc2.id()))
//     );
// }

// #[test]
// fn find_error_dynamic_to_static_one_branch() {
//     let mut graph = SoundGraph::new();

//     let proc1 = graph
//         .add_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let proc2 = graph
//         .add_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let input_id = graph
//         .add_sound_input(
//             proc1.id(),
//             InputOptions::Synchronous,
//             vec![SoundInputBranchId::new(1)],
//         )
//         .unwrap();

//     graph.connect_sound_input(input_id, proc2.id()).unwrap();

//     assert_eq!(find_sound_error(&graph), None);
// }

// #[test]
// fn find_error_dynamic_to_static_two_branches() {
//     let mut graph = SoundGraph::new();

//     let proc1 = graph
//         .add_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let proc2 = graph
//         .add_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let input_id = graph
//         .add_sound_input(
//             proc1.id(),
//             InputOptions::Synchronous,
//             vec![SoundInputBranchId::new(1), SoundInputBranchId::new(2)],
//         )
//         .unwrap();

//     graph.connect_sound_input(input_id, proc2.id()).unwrap();

//     assert_eq!(
//         find_sound_error(&graph),
//         Some(SoundError::StaticNotOneState(proc2.id()))
//     );
// }

// #[test]
// fn find_error_dynamic_to_static_nonsync() {
//     let mut graph = SoundGraph::new();

//     let proc1 = graph
//         .add_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let proc2 = graph
//         .add_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let input_id = graph
//         .add_sound_input(
//             proc1.id(),
//             InputOptions::NonSynchronous,
//             vec![SoundInputBranchId::new(1)],
//         )
//         .unwrap();

//     graph.connect_sound_input(input_id, proc2.id()).unwrap();

//     assert_eq!(
//         find_sound_error(&graph),
//         Some(SoundError::StaticNotSynchronous(proc2.id()))
//     );
// }

// #[test]
// fn find_error_dynamic_to_dynamic_to_static_no_branches() {
//     let mut graph = SoundGraph::new();

//     let proc1 = graph
//         .add_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let proc2 = graph
//         .add_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let proc3 = graph
//         .add_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let input1 = graph
//         .add_sound_input(proc1.id(), InputOptions::Synchronous, vec![])
//         .unwrap();

//     let input2 = graph
//         .add_sound_input(
//             proc2.id(),
//             InputOptions::Synchronous,
//             vec![SoundInputBranchId::new(1)],
//         )
//         .unwrap();

//     graph.connect_sound_input(input1, proc2.id()).unwrap();
//     graph.connect_sound_input(input2, proc3.id()).unwrap();

//     assert_eq!(
//         find_sound_error(&graph),
//         Some(SoundError::StaticNotOneState(proc3.id()))
//     );
// }

// #[test]
// fn find_error_dynamic_to_dynamic_to_static_one_branch() {
//     let mut graph = SoundGraph::new();

//     let proc1 = graph
//         .add_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let proc2 = graph
//         .add_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let proc3 = graph
//         .add_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let input1 = graph
//         .add_sound_input(
//             proc1.id(),
//             InputOptions::Synchronous,
//             vec![SoundInputBranchId::new(1)],
//         )
//         .unwrap();

//     let input2 = graph
//         .add_sound_input(
//             proc2.id(),
//             InputOptions::Synchronous,
//             vec![SoundInputBranchId::new(1)],
//         )
//         .unwrap();

//     graph.connect_sound_input(input1, proc2.id()).unwrap();
//     graph.connect_sound_input(input2, proc3.id()).unwrap();

//     assert_eq!(find_sound_error(&graph), None);
// }

// #[test]
// fn find_error_dynamic_to_dynamic_to_static_cycle() {
//     let mut graph = SoundGraph::new();

//     let proc1 = graph
//         .add_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let proc2 = graph
//         .add_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let proc3 = graph
//         .add_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let input1 = graph
//         .add_sound_input(
//             proc1.id(),
//             InputOptions::Synchronous,
//             vec![SoundInputBranchId::new(1)],
//         )
//         .unwrap();

//     let input2 = graph
//         .add_sound_input(
//             proc2.id(),
//             InputOptions::Synchronous,
//             vec![SoundInputBranchId::new(1)],
//         )
//         .unwrap();

//     let input3 = graph
//         .add_sound_input(
//             proc3.id(),
//             InputOptions::Synchronous,
//             vec![SoundInputBranchId::new(1)],
//         )
//         .unwrap();

//     graph.connect_sound_input(input1, proc2.id()).unwrap();
//     graph.connect_sound_input(input2, proc3.id()).unwrap();
//     graph.connect_sound_input(input3, proc1.id()).unwrap();

//     assert!(match find_sound_error(&graph) {
//         Some(SoundError::CircularDependency { cycle: _ }) => true,
//         _ => false,
//     });
// }

// #[test]
// fn find_error_dynamic_to_dynamic_to_static_two_branches() {
//     let mut graph = SoundGraph::new();

//     let proc1 = graph
//         .add_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let proc2 = graph
//         .add_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let proc3 = graph
//         .add_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let input1 = graph
//         .add_sound_input(
//             proc1.id(),
//             InputOptions::Synchronous,
//             vec![SoundInputBranchId::new(1), SoundInputBranchId::new(2)],
//         )
//         .unwrap();

//     let input2 = graph
//         .add_sound_input(
//             proc2.id(),
//             InputOptions::Synchronous,
//             vec![SoundInputBranchId::new(1)],
//         )
//         .unwrap();

//     graph.connect_sound_input(input1, proc2.id()).unwrap();
//     graph.connect_sound_input(input2, proc3.id()).unwrap();

//     assert_eq!(
//         find_sound_error(&graph),
//         Some(SoundError::StaticNotOneState(proc3.id()))
//     );
// }

// #[test]
// fn find_error_dynamic_to_dynamic_to_static_nonsync() {
//     let mut graph = SoundGraph::new();

//     let proc1 = graph
//         .add_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let proc2 = graph
//         .add_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let proc3 = graph
//         .add_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let input1 = graph
//         .add_sound_input(
//             proc1.id(),
//             InputOptions::NonSynchronous,
//             vec![SoundInputBranchId::new(1)],
//         )
//         .unwrap();

//     let input2 = graph
//         .add_sound_input(
//             proc2.id(),
//             InputOptions::Synchronous,
//             vec![SoundInputBranchId::new(1)],
//         )
//         .unwrap();

//     graph.connect_sound_input(input1, proc2.id()).unwrap();
//     graph.connect_sound_input(input2, proc3.id()).unwrap();

//     assert_eq!(
//         find_sound_error(&graph),
//         Some(SoundError::StaticNotSynchronous(proc3.id()))
//     );
// }

// #[test]
// fn find_error_dynamic_indirect_fork_to_static_nonsync() {
//     let mut graph = SoundGraph::new();

//     let proc_root1 = graph
//         .add_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let proc_root2 = graph
//         .add_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let proc_middle = graph
//         .add_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let proc_leaf = graph
//         .add_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let input_root1 = graph
//         .add_sound_input(
//             proc_root1.id(),
//             InputOptions::Synchronous,
//             vec![SoundInputBranchId::new(1)],
//         )
//         .unwrap();

//     let input_root2 = graph
//         .add_sound_input(
//             proc_root2.id(),
//             InputOptions::Synchronous,
//             vec![SoundInputBranchId::new(1)],
//         )
//         .unwrap();

//     let input_middle = graph
//         .add_sound_input(
//             proc_middle.id(),
//             InputOptions::Synchronous,
//             vec![SoundInputBranchId::new(1)],
//         )
//         .unwrap();

//     graph
//         .connect_sound_input(input_root1, proc_middle.id())
//         .unwrap();
//     graph
//         .connect_sound_input(input_root2, proc_middle.id())
//         .unwrap();
//     graph
//         .connect_sound_input(input_middle, proc_leaf.id())
//         .unwrap();

//     assert_eq!(
//         find_sound_error(&graph),
//         Some(SoundError::StaticNotOneState(proc_leaf.id()))
//     );
// }

// #[test]
// fn find_error_dynamic_direct_fork_to_static_nonsync() {
//     let mut graph = SoundGraph::new();

//     let proc_root1 = graph
//         .add_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let proc_root2 = graph
//         .add_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let proc_leaf = graph
//         .add_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let input_root1 = graph
//         .add_sound_input(
//             proc_root1.id(),
//             InputOptions::Synchronous,
//             vec![SoundInputBranchId::new(1)],
//         )
//         .unwrap();

//     let input_root2 = graph
//         .add_sound_input(
//             proc_root2.id(),
//             InputOptions::Synchronous,
//             vec![SoundInputBranchId::new(1)],
//         )
//         .unwrap();

//     graph
//         .connect_sound_input(input_root1, proc_leaf.id())
//         .unwrap();
//     graph
//         .connect_sound_input(input_root2, proc_leaf.id())
//         .unwrap();

//     assert_eq!(find_sound_error(&graph), None);
// }

// #[test]
// fn find_error_dynamic_split_to_static_two_inputs() {
//     let mut graph = SoundGraph::new();

//     let proc_root = graph
//         .add_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let proc_side = graph
//         .add_sound_processor::<TestDynamicSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let proc_leaf = graph
//         .add_sound_processor::<TestStaticSoundProcessor>(&ParsedArguments::new_empty())
//         .unwrap();

//     let input_side = graph
//         .add_sound_input(
//             proc_side.id(),
//             InputOptions::Synchronous,
//             vec![SoundInputBranchId::new(1)],
//         )
//         .unwrap();

//     let input_leaf_a = graph
//         .add_sound_input(
//             proc_leaf.id(),
//             InputOptions::Synchronous,
//             vec![SoundInputBranchId::new(1)],
//         )
//         .unwrap();

//     let input_leaf_b = graph
//         .add_sound_input(
//             proc_leaf.id(),
//             InputOptions::Synchronous,
//             vec![SoundInputBranchId::new(1)],
//         )
//         .unwrap();

//     graph
//         .connect_sound_input(input_leaf_a, proc_root.id())
//         .unwrap();
//     graph
//         .connect_sound_input(input_leaf_b, proc_side.id())
//         .unwrap();
//     graph
//         .connect_sound_input(input_side, proc_root.id())
//         .unwrap();

//     assert_eq!(find_sound_error(&graph), None);
// }
