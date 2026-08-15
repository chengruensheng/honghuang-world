//! 网格 - 定位 - 园：坐标网格 + 哈希索引定位。

use std::collections::HashMap;

use crate::{格位, 记录, 坐标层, 找格位};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// 按名字定位格位。
pub fn 按名字找格位(格位名: &str) -> Option<格位> {
    找格位(格位名)
}

/// 按坐标（层 + 对象）过滤记录。
pub fn 按坐标过滤(记录们: &[记录], 层: 坐标层, 对象: &str) -> Vec<记录> {
    记录们
        .iter()
        .filter(|记录| {
            记录.坐标
                .as_ref()
                .map(|坐标| 坐标.层 == 层 && (对象.is_empty() || 坐标.对象 == 对象))
                .unwrap_or(false)
        })
        .cloned()
        .collect()
}

/// 五级坐标层清单。
pub fn 层级清单() -> Vec<坐标层> {
    vec![坐标层::项目, 坐标层::模块, 坐标层::文件, 坐标层::符号, 坐标层::代码]
}

/// 坐标网格：某层的对象 × 属性清单。
pub struct 坐标网格 {
    pub 层: 坐标层,
    pub 对象们: Vec<&'static str>,
    pub 属性们: Vec<&'static str>,
}

/// 取某层的坐标网格（项目层无网格，返回 None）。
pub fn 取网格(层: 坐标层) -> Option<坐标网格> {
    match 层 {
        坐标层::模块 => Some(坐标网格 {
            层,
            对象们: vec!["模块", "领域", "目录", "接口", "构建", "配置", "部署", "测试"],
            属性们: 八属性(),
        }),
        坐标层::文件 => Some(坐标网格 {
            层,
            对象们: vec!["源文件", "文档", "样式", "数据", "资源", "模板", "配置", "测试", "脚本", "组件", "页面", "服务", "模型", "视图", "入口", "库"],
            属性们: 八属性(),
        }),
        坐标层::符号 => Some(坐标网格 {
            层,
            对象们: vec!["函数", "类", "方法", "属性", "变量", "常量", "类型", "接口", "事件", "钩子", "状态", "引用", "导入", "导出", "装饰", "泛型"],
            属性们: 十六属性(),
        }),
        坐标层::代码 => Some(坐标网格 {
            层,
            对象们: vec!["语句", "表达式", "调用", "赋值", "返回", "条件", "循环", "匹配", "解构", "闭包", "宏", "泛型", "模式", "字面量", "运算符", "索引", "链式", "异步", "迭代", "集合", "转换", "比较", "逻辑", "算术", "位运算", "引用", "可变", "所有权", "生命周期", "注解", "绑定", "标注"],
            属性们: 十六属性(),
        }),
        _ => None,
    }
}

/// 网格大小（对象数 × 属性数），对应 64 / 128 / 256 / 512。
pub fn 网格大小(层: 坐标层) -> Option<usize> {
    取网格(层).map(|网格| 网格.对象们.len() * 网格.属性们.len())
}

/// 八属性（模块 / 文件层）。
fn 八属性() -> Vec<&'static str> {
    vec!["规则", "目标", "约束", "实际", "依赖", "接口", "用途", "历史"]
}

/// 十六属性（符号 / 代码层）。
fn 十六属性() -> Vec<&'static str> {
    vec!["签名", "类型", "参数", "返回", "语义", "用例", "错误", "边界", "调用者", "被调用", "变更", "测试", "注释", "复杂度", "风险", "待议"]
}

/// 定位索引（哈希）：名字 → 格位、坐标 → 记录，建一次查多次。
#[derive(Clone, Debug, Default)]
pub struct 索引 {
    名字到格位: HashMap<String, 格位>,
    坐标到记录: HashMap<(坐标层, String), Vec<记录>>,
}

impl 索引 {
    /// 由格位们 + 记录们建索引。
    pub fn 建(格位们: &[格位], 记录们: &[记录]) -> 索引 {
        let 名字到格位: HashMap<String, 格位> = 格位们
            .iter()
            .cloned()
            .map(|网格| (网格.名字.clone(), 网格))
            .collect();
        let mut 坐标到记录: HashMap<(坐标层, String), Vec<记录>> = HashMap::new();
        for 记录 in 记录们 {
            if let Some(坐标) = &记录.坐标 {
                坐标到记录
                    .entry((坐标.层, 坐标.对象.clone()))
                    .or_default()
                    .push(记录.clone());
            }
        }
        索引 { 名字到格位, 坐标到记录 }
    }

    /// 名字定位格位。
    pub fn 名字定位(&self, 名字: &str) -> Option<&格位> {
        self.名字到格位.get(名字)
    }

    /// 坐标定位记录（按层 + 对象）。
    pub fn 坐标定位(&self, 层: 坐标层, 对象: &str) -> &[记录] {
        self.坐标到记录
            .get(&(层, 对象.to_string()))
            .map(|值| 值.as_slice())
            .unwrap_or(&[])
    }
}

/// 坐标索引：坐标（层·对象·属性）→ 格位名 的定位索引，落盘为 json。
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct 坐标索引 {
    pub 条目们: Vec<索引条目>,
}

/// 索引条目：坐标 → 格位名。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 索引条目 {
    pub 层: 坐标层,
    pub 对象: String,
    pub 属性: String,
    pub 格位名: String,
}

impl 坐标索引 {
    /// 登记一条坐标 → 格位名。
    pub fn 登记(&mut self, 层: 坐标层, 对象: &str, 属性: &str, 格位名: &str) {
        self.条目们.push(索引条目 {
            层,
            对象: 对象.to_string(),
            属性: 属性.to_string(),
            格位名: 格位名.to_string(),
        });
    }

    /// 按坐标（层 + 对象）查询条目。
    pub fn 查询(&self, 层: 坐标层, 对象: &str) -> Vec<&索引条目> {
        self.条目们
            .iter()
            .filter(|条目| 条目.层 == 层 && (对象.is_empty() || 条目.对象 == 对象))
            .collect()
    }

    /// 保存在工作区（.上下文/坐标索引.json）。
    pub fn 保存在工作区(&self, 工作区: &crate::工作区) -> Result<(), String> {
        self.保存(工作区.坐标索引路径())
    }

    /// 保存到指定路径。
    pub fn 保存(&self, 路径: impl AsRef<Path>) -> Result<(), String> {
        let 文本 = serde_json::to_string_pretty(self).map_err(|错误| format!("序列化坐标索引失败: {错误}"))?;
        std::fs::write(路径.as_ref(), 文本).map_err(|错误| format!("保存坐标索引失败: {错误}"))
    }

    /// 从工作区加载（.上下文/坐标索引.json）。
    pub fn 加载自工作区(工作区: &crate::工作区) -> Result<坐标索引, String> {
        Self::加载(工作区.坐标索引路径())
    }

    /// 从指定路径加载（不存在则返回空索引）。
    pub fn 加载(路径: impl AsRef<Path>) -> Result<坐标索引, String> {
        let 路径 = 路径.as_ref();
        if !路径.exists() {
            return Ok(坐标索引::default());
        }
        let 文本 = std::fs::read_to_string(路径).map_err(|错误| format!("读取坐标索引失败: {错误}"))?;
        serde_json::from_str(&文本).map_err(|错误| format!("解析坐标索引失败: {错误}"))
    }
}

#[cfg(test)]
mod 测试 {
    use super::*;
    use crate::{坐标, 记录, 范畴, 固化度, 共享度, 顺序档位};

    fn 造记录(层: 坐标层, 对象: &str) -> 记录 {
        let mut 记录 = 记录::新("结构", &format!("{层:?}/{对象}"), "测试", "代码");
        记录.坐标 = Some(坐标 { 层, 对象: 对象.to_string(), 属性: "清单".to_string() });
        记录
    }

    #[test]
    fn 按坐标过滤命中() {
        let 记录们 = vec![造记录(坐标层::文件, "源文件"), 造记录(坐标层::符号, "函数")];
        let 命中 = 按坐标过滤(&记录们, 坐标层::文件, "");
        assert_eq!(命中.len(), 1);
        assert_eq!(命中[0].坐标.as_ref().unwrap().对象, "源文件");
    }

    #[test]
    fn 网格大小对齐设计() {
        assert_eq!(网格大小(坐标层::模块), Some(64));
        assert_eq!(网格大小(坐标层::文件), Some(128));
        assert_eq!(网格大小(坐标层::符号), Some(256));
        assert_eq!(网格大小(坐标层::代码), Some(512));
        assert_eq!(网格大小(坐标层::项目), None);
    }

    #[test]
    fn 哈希索引定位() {
        let 格位们 = vec![格位::新("结构", 范畴::世界, "组织", "代码", 固化度::权, 共享度::共享, 顺序档位::中间)];
        let 记录们 = vec![造记录(坐标层::文件, "入口.rs")];
        let 索引 = 索引::建(&格位们, &记录们);
        assert!(索引.名字定位("结构").is_some());
        assert_eq!(索引.坐标定位(坐标层::文件, "入口.rs").len(), 1);
        assert!(索引.坐标定位(坐标层::符号, "入口.rs").is_empty());
    }

    #[test]
    fn 坐标索引登记查询() {
        let mut 索引 = 坐标索引::default();
        索引.登记(坐标层::文件, "源文件", "清单", "结构");
        索引.登记(坐标层::符号, "函数", "签名", "调用");
        let 命中 = 索引.查询(坐标层::文件, "");
        assert_eq!(命中.len(), 1);
        assert_eq!(命中[0].格位名, "结构");
    }

    #[test]
    fn 坐标索引保存加载() {
        let 路径 = std::env::temp_dir().join("识海测试-坐标索引.json");
        let mut 索引 = 坐标索引::default();
        索引.登记(坐标层::文件, "源文件", "清单", "结构");
        索引.保存(路径.to_str().unwrap()).unwrap();
        let 读回 = 坐标索引::加载(路径.to_str().unwrap()).unwrap();
        assert_eq!(读回.条目们.len(), 1);
        assert_eq!(读回.条目们[0].格位名, "结构");
        let _ = std::fs::remove_file(&路径);
    }
}
