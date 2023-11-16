use super::{
    ast::{ASTNode, ASTPath, VariableDefinition},
    lexicallayout::LexicalLayout,
};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub(super) enum LineLocation {
    VariableDefinition(usize),
    FinalExpression,
}

pub(super) enum LexicalLayoutCursorValue<'a> {
    AtVariableName(&'a VariableDefinition),
    AtVariableValue(&'a VariableDefinition, ASTPath),
    AtFinalExpression(&'a ASTNode, ASTPath),
}

#[derive(Clone)]
pub(super) enum LexicalLayoutCursor {
    AtVariableName(usize),
    AtVariableValue(usize, ASTPath),
    AtFinalExpression(ASTPath),
}

impl LexicalLayoutCursor {
    pub(super) fn line(&self) -> LineLocation {
        match self {
            LexicalLayoutCursor::AtVariableName(l) => LineLocation::VariableDefinition(*l),
            LexicalLayoutCursor::AtVariableValue(l, _) => LineLocation::VariableDefinition(*l),
            LexicalLayoutCursor::AtFinalExpression(_) => LineLocation::FinalExpression,
        }
    }

    pub(super) fn path(&self) -> Option<&ASTPath> {
        match self {
            LexicalLayoutCursor::AtVariableName(_) => None,
            LexicalLayoutCursor::AtVariableValue(_, p) => Some(p),
            LexicalLayoutCursor::AtFinalExpression(p) => Some(p),
        }
    }

    pub(super) fn path_mut(&mut self) -> Option<&mut ASTPath> {
        match self {
            LexicalLayoutCursor::AtVariableName(_) => None,
            LexicalLayoutCursor::AtVariableValue(_, p) => Some(p),
            LexicalLayoutCursor::AtFinalExpression(p) => Some(p),
        }
    }

    pub(super) fn go_left(&mut self, layout: &LexicalLayout) {
        let defns = layout.variable_definitions();
        match self {
            LexicalLayoutCursor::AtVariableName(i) => {
                let Some(i_prev) = i.checked_sub(1) else {
                    return;
                };
                *self = LexicalLayoutCursor::AtVariableValue(
                    i_prev,
                    ASTPath::new_at_end_of(defns[i_prev].value()),
                );
            }
            LexicalLayoutCursor::AtVariableValue(i, p) => {
                if p.is_at_beginning() {
                    *self = LexicalLayoutCursor::AtVariableName(*i);
                } else {
                    p.go_left(defns[*i].value());
                }
            }
            LexicalLayoutCursor::AtFinalExpression(p) => {
                if p.is_at_beginning() {
                    let Some(i) = defns.len().checked_sub(1) else {
                        return;
                    };
                    *self = LexicalLayoutCursor::AtVariableValue(
                        i,
                        ASTPath::new_at_end_of(defns[i].value()),
                    );
                } else {
                    p.go_left(layout.final_expression());
                }
            }
        }
    }

    pub(super) fn go_right(&mut self, layout: &LexicalLayout) {
        let defns = layout.variable_definitions();
        match self {
            LexicalLayoutCursor::AtVariableName(i) => {
                *self = LexicalLayoutCursor::AtVariableValue(*i, ASTPath::new_at_beginning());
            }
            LexicalLayoutCursor::AtVariableValue(i, p) => {
                if p.is_at_end_of(defns[*i].value()) {
                    let i_next = *i + 1;
                    if i_next == defns.len() {
                        *self = LexicalLayoutCursor::AtFinalExpression(ASTPath::new_at_beginning());
                    } else {
                        *self = LexicalLayoutCursor::AtVariableName(i_next);
                    }
                } else {
                    p.go_right(defns[*i].value());
                }
            }
            LexicalLayoutCursor::AtFinalExpression(p) => {
                p.go_right(layout.final_expression());
            }
        }
    }

    pub(super) fn go_up(&mut self, layout: &LexicalLayout) {
        match self {
            LexicalLayoutCursor::AtVariableName(i) => {
                *self = LexicalLayoutCursor::AtVariableName(i.saturating_sub(1))
            }
            LexicalLayoutCursor::AtVariableValue(i, _) => {
                *self = LexicalLayoutCursor::AtVariableValue(
                    i.saturating_sub(1),
                    ASTPath::new_at_beginning(),
                );
            }
            LexicalLayoutCursor::AtFinalExpression(_) => {
                let Some(i) = layout.variable_definitions().len().checked_sub(1) else {
                    *self = LexicalLayoutCursor::AtFinalExpression(ASTPath::new_at_beginning());
                    return;
                };
                *self = LexicalLayoutCursor::AtVariableValue(i, ASTPath::new_at_beginning());
            }
        }
    }

    pub(super) fn go_down(&mut self, layout: &LexicalLayout) {
        match self {
            LexicalLayoutCursor::AtVariableName(i) => {
                let i_next = *i + 1;
                if i_next == layout.variable_definitions().len() {
                    *self = LexicalLayoutCursor::AtFinalExpression(ASTPath::new_at_beginning());
                } else {
                    *self = LexicalLayoutCursor::AtVariableName(i_next);
                }
            }
            LexicalLayoutCursor::AtVariableValue(i, _) => {
                let i_next = *i + 1;
                if i_next == layout.variable_definitions().len() {
                    *self = LexicalLayoutCursor::AtFinalExpression(ASTPath::new_at_beginning());
                } else {
                    *self =
                        LexicalLayoutCursor::AtVariableValue(i_next, ASTPath::new_at_beginning());
                }
            }
            LexicalLayoutCursor::AtFinalExpression(_) => {
                *self = LexicalLayoutCursor::AtFinalExpression(ASTPath::new_at_end_of(
                    layout.final_expression(),
                ));
            }
        }
    }

    pub(super) fn get<'a>(&self, layout: &'a LexicalLayout) -> LexicalLayoutCursorValue<'a> {
        match self {
            LexicalLayoutCursor::AtVariableName(i) => {
                LexicalLayoutCursorValue::AtVariableName(&layout.variable_definitions()[*i])
            }
            LexicalLayoutCursor::AtVariableValue(i, p) => {
                LexicalLayoutCursorValue::AtVariableValue(
                    &layout.variable_definitions()[*i],
                    p.clone(),
                )
            }
            LexicalLayoutCursor::AtFinalExpression(p) => {
                LexicalLayoutCursorValue::AtFinalExpression(&layout.final_expression(), p.clone())
            }
        }
    }

    pub(super) fn get_node<'a>(&self, layout: &'a LexicalLayout) -> Option<&'a ASTNode> {
        match self {
            LexicalLayoutCursor::AtVariableName(_) => None,
            LexicalLayoutCursor::AtVariableValue(i, p) => Some(
                layout.variable_definitions()[*i]
                    .value()
                    .get_along_path(p.steps()),
            ),
            LexicalLayoutCursor::AtFinalExpression(p) => {
                Some(layout.final_expression().get_along_path(p.steps()))
            }
        }
    }

    pub(super) fn get_variables_in_scope<'a>(
        &self,
        layout: &'a LexicalLayout,
    ) -> &'a [VariableDefinition] {
        match self {
            LexicalLayoutCursor::AtVariableName(i) => &layout.variable_definitions()[..(*i)],
            LexicalLayoutCursor::AtVariableValue(i, _) => &layout.variable_definitions()[..(*i)],
            LexicalLayoutCursor::AtFinalExpression(_) => layout.variable_definitions(),
        }
    }

    pub(super) fn set_node(&self, layout: &mut LexicalLayout, value: ASTNode) {
        let (node, path) = match self {
            LexicalLayoutCursor::AtVariableName(_) => {
                panic!("Can't set a node value when the cursor is pointing at a variable name")
            }
            LexicalLayoutCursor::AtVariableValue(i, p) => {
                (layout.variable_definitions_mut()[*i].value_mut(), p)
            }
            LexicalLayoutCursor::AtFinalExpression(p) => (layout.final_expression_mut(), p),
        };

        node.set_along_path(path.steps(), value);
    }

    pub(super) fn go_to_final_expression(&mut self) {
        *self = LexicalLayoutCursor::AtFinalExpression(ASTPath::new_at_beginning());
    }
}
