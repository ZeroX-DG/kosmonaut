use crate::dom::tree::{NodeData, NodeRef};
use crate::layout::behavior::BaseLayoutBoxBehavior;
use crate::layout::flow::block::{AnonymousBlockBox, BlockLevelBox};
use crate::layout::flow::inline::{InlineBox, TextRun};
use crate::layout::formatting_context::{
    FormattingContext, FormattingContextRef, QualifiedFormattingContext,
};
use crate::layout::layout_box::LayoutBox;
use crate::style::values::computed::display::{DisplayBox, InnerDisplay, OuterDisplay};
use crate::style::values::computed::Display;

/// Takes a DOM node and builds the corresponding box tree of it and its children.  Returns
/// `None` if `node` is a `Display::None`.
///
/// In Kosmonaut, the terms "box tree" and "layout tree" are not the same in that the layout tree
/// represents the box tree post-layout (sizes and positions assigned). However, this distinction
/// isn't always held up, and sometimes "box tree" and "layout tree" are used interchangeably.
///
/// `parent_context` is the formatting context which the boxes generated by this node should join
/// (assuming the boxes generated by this node don't establish their own formatting context.)
///
/// If this is `None`, that means the boxes generated by the given `node` are expected to generate a
/// new formatting context.
pub fn build_box_tree(
    node: NodeRef,
    parent_context: Option<FormattingContextRef>,
) -> Option<LayoutBox> {
    if let NodeData::Document(_) = node.data() {
        // We don't want to create boxes for the document node nor the doctype nodes, so skip past
        // them to the root <html> element and start building the box tree there.
        //
        // Here's what the box tree would like if we didn't do this (which would be incorrect):
        //
        // DOCUMENT Inline LayoutBox at (0, 0) size 1920x184  --- Bad
        //   DOCTYPE Inline LayoutBox at (0, 0) size 1920x0   --- Bad
        //   HTML Block LayoutBox at (0, 0) size 1920x184
        //     BODY Block LayoutBox at (8, 8) size 1904x168
        //       ...
        return node
            .children()
            .find(|child| match child.data() {
                NodeData::Element(data) => local_name!("html") == data.name.local,
                _ => false,
            })
            .map(|html_node| build_box_tree(html_node, None))
            .flatten();
    }

    let mut layout_box = if let NodeData::Text(text) = node.data() {
        // https://drafts.csswg.org/css-display-3/#flow-layout
        // > If the [text] sequence contains no text, however, it does not generate a text run.
        let contents = text.clone().take().trim().to_owned();
        if contents.is_empty() {
            return None;
        }
        let pfc = parent_context.unwrap();
        assert!(pfc.is_inline_formatting_context());
        TextRun::new(node.clone(), pfc, contents).into()
    } else {
        match build_box_from_display(node.clone(), parent_context) {
            Some(layout_box) => layout_box,
            None => return None,
        }
    };

    for child in node.children() {
        if let NodeData::Text(text) = child.data() {
            // https://drafts.csswg.org/css-display-3/#flow-layout
            // > If the [text] sequence contains no text, however, it does not generate a text run.
            let contents = text.clone().take().trim().to_owned();
            if contents.is_empty() {
                continue;
            }

            // Get (or create, if necessary) an inline container for this new text-run.
            let inline_container = get_or_create_inline_container(&mut layout_box, child.clone());
            inline_container.add_child(
                TextRun::new(
                    child.clone(),
                    inline_container.formatting_context(),
                    text.clone().take().trim().to_owned(),
                )
                .into(),
            );
            continue;
        }
        handle_child_node_by_display(&mut layout_box, child);
    }
    Some(layout_box)
}

fn handle_child_node_by_display(parent_box: &mut LayoutBox, child_node: NodeRef) {
    let child_computed_values = &*child_node.computed_values();
    match child_computed_values.display {
        Display::Full(full_display) => {
            match (full_display.outer(), full_display.inner()) {
                (OuterDisplay::Block, InnerDisplay::Flow)
                | (OuterDisplay::Block, InnerDisplay::FlowRoot) => {
                    if let Some(child_box) =
                        build_box_tree(child_node.clone(), Some(parent_box.formatting_context()))
                    {
                        // TODO: We don't handle the case where a block-flow child box is added to an inline box.
                        // This current behavior is wrong — we should be checking if `node` is an `Display::Inline` and
                        // doing something different here.  To fix, see: https://www.w3.org/TR/CSS2/visuren.html#box-gen
                        // Namely, the paragraph that begins with "When an inline box contains an in-flow block-level box"
                        // This concept _might_ be called "fragmenting".
                        parent_box.add_child(child_box)
                    }
                }
                (OuterDisplay::Inline, InnerDisplay::Flow) => {
                    let inline_container =
                        get_or_create_inline_container(parent_box, child_node.clone());
                    if let Some(child_box) = build_box_tree(
                        child_node.clone(),
                        Some(inline_container.formatting_context()),
                    ) {
                        inline_container.add_child(child_box)
                    }
                }
                (OuterDisplay::Inline, InnerDisplay::FlowRoot) => unimplemented!(),
            }
        }
        Display::Box(DisplayBox::None) => {}
    }
}

fn build_box_from_display(
    node: NodeRef,
    parent_context: Option<FormattingContextRef>,
) -> Option<LayoutBox> {
    let computed_values = node.computed_values();
    // Per the "Generated box" column from the table in this section, decide what boxes to generate
    // from this DOM node.  https://drafts.csswg.org/css-display/#the-display-properties
    Some(match computed_values.display {
        Display::Full(full_display) => {
            match (full_display.outer(), full_display.inner()) {
                (OuterDisplay::Block, InnerDisplay::Flow) => {
                    // Per https://www.w3.org/TR/css-display-3/#block-container, join this new block
                    // container with our parent formatting context if it is a BFC.
                    let formatting_context = match parent_context.clone() {
                        Some(rc_qfc) => {
                            match *rc_qfc {
                                QualifiedFormattingContext::Independent(
                                    FormattingContext::Block,
                                )
                                | QualifiedFormattingContext::Dependent(FormattingContext::Block) => {
                                    parent_context.unwrap()
                                }
                                // Parent formatting context is not a BFC, create a new one instead.
                                _ => FormattingContextRef::new_independent_block(),
                            }
                        }
                        // There is no parent formatting context -- create a new BFC.
                        _ => FormattingContextRef::new_independent_block(),
                    };
                    BlockLevelBox::new_block_container(node.clone(), formatting_context).into()
                }
                (OuterDisplay::Block, InnerDisplay::FlowRoot) => {
                    BlockLevelBox::new_block_container(
                        node.clone(),
                        FormattingContextRef::new_independent_block(),
                    )
                    .into()
                }
                (OuterDisplay::Inline, InnerDisplay::Flow) => {
                    let formatting_context = match parent_context.clone() {
                        Some(rc_qfc) => {
                            match *rc_qfc {
                                QualifiedFormattingContext::Independent(FormattingContext::Inline) |
                                QualifiedFormattingContext::Dependent(FormattingContext::Inline) => {
                                    parent_context.unwrap()
                                }
                                _ => panic!("when trying to add inline box, parent formatting context was present but it wasn't an inline formatting context")
                            }
                        }
                        _ => {
                            panic!("there was no parent formatting context to add inline box to")
                        }
                    };
                    InlineBox::new(node.clone(), formatting_context).into()
                }
                (OuterDisplay::Inline, InnerDisplay::FlowRoot) => unimplemented!(),
            }
        }
        Display::Box(DisplayBox::None) => return None,
    })
}

fn get_or_create_inline_container(
    layout_box: &mut LayoutBox,
    node_for_container: NodeRef,
) -> &mut LayoutBox {
    if layout_box.get_mut_inline_container().is_none() {
        layout_box.add_child(create_inline_container(node_for_container));
    }
    // TODO: There must be another way to get the anonymous inline box we just added.
    // This could cause poor runtime performance for boxes with a lot of children, but works for now.
    // Maybe we'd need to use RefCell in order to get this kind of interior mutability?
    layout_box.get_mut_inline_container().unwrap()
}

fn create_inline_container(node: NodeRef) -> LayoutBox {
    // Create a new IFC for this inline content.
    let mut anonymous_block_box =
        AnonymousBlockBox::new(node.clone(), FormattingContextRef::new_independent_inline());
    anonymous_block_box.add_child(LayoutBox::create_root_inline_box(
        node,
        anonymous_block_box.formatting_context(),
    ));
    anonymous_block_box.into()
}
