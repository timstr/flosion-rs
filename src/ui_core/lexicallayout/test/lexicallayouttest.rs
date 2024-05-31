use crate::{
    core::expression::{
        expressiongraph::ExpressionGraphParameterId, expressionnode::ExpressionNodeId,
    },
    ui_core::lexicallayout::ast::{
        ASTNode, ASTNodeValue, ASTPath, InternalASTNode, InternalASTNodeValue, VariableId,
    },
};

fn create_test_ast() -> ASTNode {
    ASTNode::new(ASTNodeValue::Internal(Box::new(InternalASTNode::new(
        InternalASTNodeValue::Function(
            ExpressionNodeId::new(1),
            vec![
                ASTNode::new(ASTNodeValue::Empty),
                ASTNode::new(ASTNodeValue::Internal(Box::new(InternalASTNode::new(
                    InternalASTNodeValue::Function(
                        ExpressionNodeId::new(2),
                        vec![ASTNode::new(ASTNodeValue::Variable(VariableId::new(1)))],
                    ),
                )))),
                ASTNode::new(ASTNodeValue::GraphInput(ExpressionGraphParameterId::new(
                    11,
                ))),
                ASTNode::new(ASTNodeValue::Variable(VariableId::new(2))),
            ],
        ),
    ))))
}

#[test]
fn test_get_along_path() {
    let tree = create_test_ast();

    let ASTNodeValue::Internal(tree_at_empty_path) = tree.get_along_path(&[]).value() else {
        panic!();
    };

    assert!(
        if let InternalASTNodeValue::Function(_, _) = tree_at_empty_path.value() {
            true
        } else {
            false
        }
    );

    assert!(match tree.get_along_path(&[0]).value() {
        ASTNodeValue::Empty => true,
        _ => false,
    });

    let ASTNodeValue::Internal(tree_at_path_1) = tree.get_along_path(&[1]).value() else {
        panic!();
    };

    assert!(
        if let InternalASTNodeValue::Function(_, _) = tree_at_path_1.value() {
            true
        } else {
            false
        }
    );

    assert!(match tree.get_along_path(&[1, 0]).value() {
        ASTNodeValue::Variable(id) => *id == VariableId::new(1),
        _ => false,
    });

    assert!(match tree.get_along_path(&[2]).value() {
        ASTNodeValue::GraphInput(giid) => *giid == ExpressionGraphParameterId::new(11),
        _ => false,
    });

    assert!(match tree.get_along_path(&[3]).value() {
        ASTNodeValue::Variable(id) => *id == VariableId::new(2),
        _ => false,
    });
}

#[test]
fn test_go_left() {
    let tree = create_test_ast();

    let mut path = ASTPath::new(vec![]);
    path.go_left(&tree);
    assert_eq!(&path.steps(), &[]);

    let mut path = ASTPath::new(vec![0]);
    path.go_left(&tree);
    assert_eq!(&path.steps(), &[]);

    let mut path = ASTPath::new(vec![1]);
    path.go_left(&tree);
    assert_eq!(&path.steps(), &[0]);
    path.go_left(&tree);
    assert_eq!(&path.steps(), &[]);

    let mut path = ASTPath::new(vec![1, 0]);
    path.go_left(&tree);
    assert_eq!(&path.steps(), &[1]);
    path.go_left(&tree);
    assert_eq!(&path.steps(), &[0]);
    path.go_left(&tree);
    assert_eq!(&path.steps(), &[]);

    let mut path = ASTPath::new(vec![3]);
    path.go_left(&tree);
    assert_eq!(&path.steps(), &[2]);
    path.go_left(&tree);
    assert_eq!(&path.steps(), &[1, 0]);
    path.go_left(&tree);
    assert_eq!(&path.steps(), &[1]);
    path.go_left(&tree);
    assert_eq!(&path.steps(), &[0]);
    path.go_left(&tree);
    assert_eq!(&path.steps(), &[]);
}

#[test]
fn test_go_right() {
    let tree = create_test_ast();

    let mut path = ASTPath::new(vec![]);
    path.go_right(&tree);
    assert_eq!(&path.steps(), &[0]);
    path.go_right(&tree);
    assert_eq!(&path.steps(), &[1]);
    path.go_right(&tree);
    assert_eq!(&path.steps(), &[1, 0]);
    path.go_right(&tree);
    assert_eq!(&path.steps(), &[2,]);
    path.go_right(&tree);
    assert_eq!(&path.steps(), &[3]);
    path.go_right(&tree);
    assert_eq!(&path.steps(), &[3]);
}
