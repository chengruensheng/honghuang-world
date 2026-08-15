//! 号令 bin 入口：极薄转发，逻辑在库 入口执行
#![allow(non_snake_case)]

use mingling_fu::执行;

fn main() {
    std::process::exit(执行());
}
