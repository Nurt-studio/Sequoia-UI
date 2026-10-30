#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::cell::RefCell;
use std::rc::Rc;

use sequoia_test::Harness;
use sequoia_view::prelude::*;

#[test]
fn tree_arrow_click_collapses_fresh_nodes() {
    let labels = Rc::new(RefCell::new(Vec::new()));
    let seen = labels.clone();
    let tree: TreeView<String> = TreeView::new(|| {
        vec![TreeNode::branch(
            "root",
            "Root".to_string(),
            vec![TreeNode::leaf("child", "Child".to_string())],
        )]
    })
    .row(move |v: &String, _| {
        seen.borrow_mut().push(v.clone());
        Text::new(v.clone())
    });
    let h = Harness::new(tree);
    h.sync(300.0, 200.0);
    assert!(labels.borrow().iter().any(|t| t == "Child"));

    labels.borrow_mut().clear();
    h.click(8.0, 14.0);
    h.sync(300.0, 200.0);
    assert!(
        !labels.borrow().iter().any(|t| t == "Child"),
        "arrow click should collapse the branch"
    );

    labels.borrow_mut().clear();
    h.click(8.0, 14.0);
    h.sync(300.0, 200.0);
    assert!(
        labels.borrow().iter().any(|t| t == "Child"),
        "a second arrow click should expand it again"
    );
}
