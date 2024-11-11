use hashstash::{InplaceUnstasher, Stashable, Stasher, UnstashError, UnstashableInplace};
use inkwell::values::FloatValue;

use crate::{
    core::{
        expression::{
            expressiongraph::ExpressionGraph,
            expressiongraph::ExpressionTarget,
            expressiongrapherror::ExpressionError,
            expressiongraphvalidation::find_expression_error,
            expressioninput::ExpressionInput,
            expressionnode::{
                ExpressionNodeVisitor, ExpressionNodeVisitorMut, ExpressionNodeWithId,
                PureExpressionNode,
            },
        },
        jit::jit::Jit,
        objecttype::{ObjectType, WithObjectType},
        stashing::StashingContext,
    },
    ui_core::arguments::ParsedArguments,
};

struct TestExpressionNode {
    input1: ExpressionInput,
    input2: ExpressionInput,
}

impl PureExpressionNode for TestExpressionNode {
    fn new(_args: &ParsedArguments) -> Self {
        TestExpressionNode {
            input1: ExpressionInput::new(0.0),
            input2: ExpressionInput::new(0.0),
        }
    }

    fn compile<'ctx>(&self, jit: &mut Jit<'ctx>, inputs: &[FloatValue<'ctx>]) -> FloatValue<'ctx> {
        debug_assert_eq!(inputs.len(), 2);
        jit.builder()
            .build_float_add(inputs[0], inputs[1], "sum")
            .unwrap()
    }

    fn visit(&self, visitor: &mut dyn ExpressionNodeVisitor) {
        visitor.input(&self.input1);
        visitor.input(&self.input2);
    }

    fn visit_mut(&mut self, visitor: &mut dyn ExpressionNodeVisitorMut) {
        visitor.input(&mut self.input1);
        visitor.input(&mut self.input2);
    }
}

impl WithObjectType for TestExpressionNode {
    const TYPE: ObjectType = ObjectType::new("testnode");
}

impl Stashable<StashingContext> for TestExpressionNode {
    fn stash(&self, stasher: &mut Stasher<StashingContext>) {
        stasher.object(&self.input1);
        stasher.object(&self.input2);
    }
}

impl UnstashableInplace for TestExpressionNode {
    fn unstash_inplace(&mut self, unstasher: &mut InplaceUnstasher) -> Result<(), UnstashError> {
        unstasher.object_inplace(&mut self.input1)?;
        unstasher.object_inplace(&mut self.input2)?;
        Ok(())
    }
}

#[test]
fn test_expression_error_empty_graph() {
    let graph = ExpressionGraph::new();

    assert_eq!(find_expression_error(&graph), None);
}

#[test]
fn test_expression_error_one_node() {
    let mut graph = ExpressionGraph::new();

    graph.add_expression_node(Box::new(
        ExpressionNodeWithId::<TestExpressionNode>::new_default(),
    ));

    assert_eq!(find_expression_error(&graph), None);
}

#[test]
fn test_expression_error_two_nodes_doubly_connected() {
    let mut graph = ExpressionGraph::new();

    let node1 = ExpressionNodeWithId::<TestExpressionNode>::new_default();

    let mut node2 = ExpressionNodeWithId::<TestExpressionNode>::new_default();
    node2
        .input1
        .set_target(Some(ExpressionTarget::Node(node1.id())));
    node2
        .input2
        .set_target(Some(ExpressionTarget::Node(node1.id())));

    graph.add_expression_node(Box::new(node1));
    graph.add_expression_node(Box::new(node2));

    assert_eq!(find_expression_error(&graph), None);
}

#[test]
fn test_expression_error_one_parameter() {
    let mut graph = ExpressionGraph::new();

    graph.add_parameter();

    assert_eq!(find_expression_error(&graph), None);
}

#[test]
fn test_expression_error_one_parameter_connected() {
    let mut graph = ExpressionGraph::new();

    let param_id = graph.add_parameter();

    graph.add_result(0.0);
    graph.results_mut()[0].set_target(Some(ExpressionTarget::Parameter(param_id)));

    assert_eq!(find_expression_error(&graph), None);
}

#[test]
fn test_expression_error_one_node_one_parameter_connected() {
    let mut graph = ExpressionGraph::new();

    let param_id = graph.add_parameter();

    let mut node = ExpressionNodeWithId::<TestExpressionNode>::new_default();
    node.input1
        .set_target(Some(ExpressionTarget::Parameter(param_id)));

    graph.add_result(0.0);
    graph.results_mut()[0].set_target(Some(ExpressionTarget::Node(node.id())));

    graph.add_expression_node(Box::new(node));

    assert_eq!(find_expression_error(&graph), None);
}

#[test]
fn test_expression_error_one_node_one_parameter_doubly_connected() {
    let mut graph = ExpressionGraph::new();

    let param_id = graph.add_parameter();

    let mut node = ExpressionNodeWithId::<TestExpressionNode>::new_default();
    node.input1
        .set_target(Some(ExpressionTarget::Parameter(param_id)));
    node.input2
        .set_target(Some(ExpressionTarget::Parameter(param_id)));

    graph.add_result(0.0);
    graph.results_mut()[0].set_target(Some(ExpressionTarget::Node(node.id())));

    graph.add_expression_node(Box::new(node));

    assert_eq!(find_expression_error(&graph), None);
}

#[test]
fn test_expression_error_two_nodes_one_parameter_disconnected() {
    let mut graph = ExpressionGraph::new();

    graph.add_parameter();

    let node1 = ExpressionNodeWithId::<TestExpressionNode>::new_default();

    let mut node2 = ExpressionNodeWithId::<TestExpressionNode>::new_default();
    node2
        .input1
        .set_target(Some(ExpressionTarget::Node(node1.id())));
    node2
        .input2
        .set_target(Some(ExpressionTarget::Node(node1.id())));

    graph.add_result(0.0);
    graph.results_mut()[0].set_target(Some(ExpressionTarget::Node(node2.id())));

    graph.add_expression_node(Box::new(node1));
    graph.add_expression_node(Box::new(node2));

    assert_eq!(find_expression_error(&graph), None);
}

#[test]
fn test_expression_error_two_nodes_one_parameter_connected() {
    let mut graph = ExpressionGraph::new();

    let param_id = graph.add_parameter();

    let mut node1 = ExpressionNodeWithId::<TestExpressionNode>::new_default();
    node1
        .input1
        .set_target(Some(ExpressionTarget::Parameter(param_id)));

    let mut node2 = ExpressionNodeWithId::<TestExpressionNode>::new_default();
    node2
        .input1
        .set_target(Some(ExpressionTarget::Node(node1.id())));
    node2
        .input2
        .set_target(Some(ExpressionTarget::Node(node1.id())));

    graph.add_result(0.0);

    graph.add_expression_node(Box::new(node1));
    graph.add_expression_node(Box::new(node2));

    assert_eq!(find_expression_error(&graph), None);
}
#[test]
fn test_expression_error_two_nodes_one_parameter_connected_with_result() {
    let mut graph = ExpressionGraph::new();

    let param_id = graph.add_parameter();

    let mut node1 = ExpressionNodeWithId::<TestExpressionNode>::new_default();
    node1
        .input1
        .set_target(Some(ExpressionTarget::Parameter(param_id)));

    let mut node2 = ExpressionNodeWithId::<TestExpressionNode>::new_default();
    node2
        .input1
        .set_target(Some(ExpressionTarget::Node(node1.id())));
    node2
        .input2
        .set_target(Some(ExpressionTarget::Node(node1.id())));

    graph.add_result(0.0);
    graph.results_mut()[0].set_target(Some(ExpressionTarget::Node(node2.id())));

    graph.add_expression_node(Box::new(node1));
    graph.add_expression_node(Box::new(node2));

    assert_eq!(find_expression_error(&graph), None);
}

#[test]
fn test_expression_error_one_node_cycle() {
    let mut graph = ExpressionGraph::new();

    let mut node = ExpressionNodeWithId::<TestExpressionNode>::new_default();
    let own_id = node.id();
    node.input1.set_target(Some(ExpressionTarget::Node(own_id)));

    graph.add_expression_node(Box::new(node));

    assert_eq!(
        find_expression_error(&graph),
        Some(ExpressionError::CircularDependency)
    );
}
