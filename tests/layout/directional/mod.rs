use crate::layout::{dump_layout_cmd, DumpLayoutVerbosity};
use insta::assert_snapshot;

#[test]
fn ltr_vertical_lr_block_boxes() {
    let mut dump_layout_cmd = dump_layout_cmd(DumpLayoutVerbosity::NonVerbose);
    dump_layout_cmd
        .arg("--files")
        .arg("tests/websrc/directional/ltr-vertical-lr-block-boxes.html")
        .arg("tests/websrc/directional/ltr-vertical-lr-block-boxes.css")
        .succeeds();
    assert_snapshot!(dump_layout_cmd.stdout());
}

#[test]
fn ltr_vertical_lr_block_boxes_bottom_right_mbp_applied_physically() {
    let mut dump_layout_cmd = dump_layout_cmd(DumpLayoutVerbosity::Verbose);
    dump_layout_cmd
        .arg("--files")
        .arg("tests/websrc/directional/ltr-vertical-lr-block-boxes-bottom-right-mbp-applied-physically.html")
        .arg("tests/websrc/directional/ltr-vertical-lr-block-boxes-bottom-right-mbp-applied-physically.css")
        .succeeds();
    assert_snapshot!(dump_layout_cmd.stdout());
}

#[test]
fn ltr_vertical_lr_block_boxes_top_left_right_mbp_applied_physically() {
    let mut dump_layout_cmd = dump_layout_cmd(DumpLayoutVerbosity::Verbose);
    dump_layout_cmd
        .arg("--files")
        .arg("tests/websrc/directional/ltr-vertical-lr-block-boxes-top-left-mbp-applied-physically.html")
        .arg("tests/websrc/directional/ltr-vertical-lr-block-boxes-top-left-mbp-applied-physically.css")
        .succeeds();
    assert_snapshot!(dump_layout_cmd.stdout());
}

#[test]
fn rtl_horizontal_tb_block_boxes() {
    let mut dump_layout_cmd = dump_layout_cmd(DumpLayoutVerbosity::NonVerbose);
    dump_layout_cmd
        .arg("--files")
        .arg("tests/websrc/directional/rtl-horizontal-tb-block-boxes.html")
        .arg("tests/websrc/directional/rtl-horizontal-tb-block-boxes.css")
        .succeeds();
    assert_snapshot!(dump_layout_cmd.stdout());
}
