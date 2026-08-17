//! 模型连接-府 · 核心类型：模型请求与响应。

use serde::{Deserialize, Serialize};

/// 一条对话消息。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 对话消息 {
    pub 角色: String,
    pub 内容: String,
    /// assistant 消息回传模型上次的工具调用（工具循环用）；纯文本对话为 None。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub 工具调用们: Option<Vec<工具调用>>,
    /// tool 角色消息回传执行结果对应的调用标识；其他角色为 None。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub 工具调用标识: Option<String>,
}

impl 对话消息 {
    /// 构造一条用户消息。
    pub fn 用户(内容: impl Into<String>) -> 对话消息 {
        对话消息 { 角色: "user".to_string(), 内容: 内容.into(), 工具调用们: None, 工具调用标识: None }
    }

    /// 构造一条系统消息。
    pub fn 系统(内容: impl Into<String>) -> 对话消息 {
        对话消息 { 角色: "system".to_string(), 内容: 内容.into(), 工具调用们: None, 工具调用标识: None }
    }

    /// assistant 消息：回传本轮工具调用，内容保留模型原回复（含 think，官方要求多轮保留完整 assistant 消息）。
    pub fn 助手_带工具调用(内容: impl Into<String>, 调用们: Vec<工具调用>) -> 对话消息 {
        对话消息 { 角色: "assistant".to_string(), 内容: 内容.into(), 工具调用们: Some(调用们), 工具调用标识: None }
    }

    /// tool 消息：回传某次工具调用的执行结果。
    pub fn 工具结果(标识: impl Into<String>, 结果: impl Into<String>) -> 对话消息 {
        对话消息 { 角色: "tool".to_string(), 内容: 结果.into(), 工具调用们: None, 工具调用标识: Some(标识.into()) }
    }
}

/// 工具定义：函数名 + 描述 + 参数 JSON Schema（OpenAI 兼容 function calling）。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 工具定义 {
    pub 名字: String,
    pub 描述: String,
    pub 参数: serde_json::Value,
}

/// 工具调用：模型请求执行一个工具。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct 工具调用 {
    /// 模型返回的调用标识（tool_call_id），回传执行结果时须一致。
    pub 标识: String,
    pub 名字: String,
    pub 参数: String,
}

/// 模型回复：文本内容或工具调用。
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum 模型回复 {
    文本(String),
    /// 工具调用，附模型原回复内容（含 think，多轮回传须保留）。
    工具调用(String, Vec<工具调用>),
    /// 工具调用参数缺失/非法（arguments 为空、{} 或非法 JSON），携带缺失参数的工具名；由上层引导模型重发完整参数。
    参数缺失(Vec<String>),
}

/// 一次模型调用的 token 用量（含缓存命中，便于成本观测与对账）。
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct 用量 {
    /// 提示词 token 数。
    pub 提示词: u64,
    /// 输出 token 数。
    pub 输出: u64,
    /// 缓存命中 token 数（cache_read_input_tokens / prompt_tokens_details.cached_tokens）。
    pub 缓存命中: u64,
    /// 总 token 数。
    pub 总计: u64,
}

impl 用量 {
    /// 累计另一次用量（多轮 / 多任务聚合用）。
    pub fn 加(&mut self, 其他: &用量) {
        self.提示词 += 其他.提示词;
        self.输出 += 其他.输出;
        self.缓存命中 += 其他.缓存命中;
        self.总计 += 其他.总计;
    }
}

#[cfg(test)]
mod 测试 {
    //! 用量累加接口契约测试：验证提示词/输出/缓存命中/总计四字段累加语义。
    use super::用量;

    #[test]
    fn 用量加法零值累加等于原值() {
        // 零值兜底：任意非零用量 + 用量::default() 四字段不变；反向零 + 非零也保持非零。
        let mut 累加 = 用量 {
            提示词: 100,
            输出: 50,
            缓存命中: 30,
            总计: 180,
        };
        累加.加(&用量::default());
        assert_eq!(累加.提示词, 100, "非零 + 零：提示词不变");
        assert_eq!(累加.输出, 50, "非零 + 零：输出不变");
        assert_eq!(累加.缓存命中, 30, "非零 + 零：缓存命中不变");
        assert_eq!(累加.总计, 180, "非零 + 零：总计不变");
        // 反向：零用量 + 非零用量 = 非零用量（四字段逐项不变）。
        let mut 累加2 = 用量::default();
        let 非零 = 用量 {
            提示词: 7,
            输出: 11,
            缓存命中: 3,
            总计: 18,
        };
        累加2.加(&非零);
        assert_eq!(累加2, 非零, "零 + 非零：四字段全部继承非零值");
    }

    #[test]
    fn 用量加法四字段逐项累加() {
        // 对称累加：A.加(&B) 四字段 = A.x + B.x（提示词/输出/缓存命中/总计均独立累加，不串扰）。
        let mut a = 用量 {
            提示词: 100,
            输出: 50,
            缓存命中: 30,
            总计: 180,
        };
        let b = 用量 {
            提示词: 200,
            输出: 80,
            缓存命中: 50,
            总计: 330,
        };
        a.加(&b);
        assert_eq!(a.提示词, 300, "提示词 = 100+200");
        assert_eq!(a.输出, 130, "输出 = 50+80");
        assert_eq!(a.缓存命中, 80, "缓存命中 = 30+50");
        assert_eq!(a.总计, 510, "总计 = 180+330");
        // 跨字段独立：单字段差异不应影响其他字段累加（提示词不动、输出缓存命中总计变化）。
        let mut c = 用量 {
            提示词: 5,
            输出: 0,
            缓存命中: 0,
            总计: 5,
        };
        let d = 用量 {
            提示词: 0,
            输出: 9,
            缓存命中: 4,
            总计: 13,
        };
        c.加(&d);
        assert_eq!(c.提示词, 5, "跨字段：提示词 5+0=5");
        assert_eq!(c.输出, 9, "跨字段：输出 0+9=9");
        assert_eq!(c.缓存命中, 4, "跨字段：缓存命中 0+4=4");
        assert_eq!(c.总计, 18, "跨字段：总计 5+13=18");
    }

    #[test]
    fn 用量加法链式累加多轮聚合() {
        // 链式累加：A.加(&B).加(&C) 四字段 = A+B+C（多任务聚合场景典型用法）。
        let mut a = 用量 {
            提示词: 10,
            输出: 5,
            缓存命中: 2,
            总计: 17,
        };
        let b = 用量 {
            提示词: 100,
            输出: 50,
            缓存命中: 20,
            总计: 170,
        };
        let c = 用量 {
            提示词: 1000,
            输出: 500,
            缓存命中: 200,
            总计: 1700,
        };
        a.加(&b);
        a.加(&c);
        assert_eq!(a.提示词, 1110, "链式：10+100+1000");
        assert_eq!(a.输出, 555, "链式：5+50+500");
        assert_eq!(a.缓存命中, 222, "链式：2+20+200");
        assert_eq!(a.总计, 1887, "链式：17+170+1700");
    }
}
