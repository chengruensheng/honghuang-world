use std::sync::{Arc, Mutex};
use hm_cognition::{心智地图, 过程上下文, 图谱, 认知注入};
use hm_content::LLM池;
use hm_log::运行日志记录器;

/// 数据服务（hm-http）共享状态：仅承载 HTTP 适配层自身持有的认知三态、日志记录器、LLM 池与鉴权令牌。
///
/// 五引擎仓储由各引擎府的路由片段自持；看板/开发端子状态归 hm-agent 的「开发服务状态」。
/// 由启动入口从装配体取出字段注入，供 axum 处理器读取。
#[derive(Clone)]
pub struct 数据服务状态 {
    pub 图谱: Arc<Mutex<图谱>>,
    pub 心智地图: Arc<Mutex<心智地图>>,
    pub 语境: Arc<Mutex<过程上下文>>,
    /// 三态认知注入（未装配时 None，认知问答接口返回未装配；装配后供检索决策/答复注入）
    pub 认知注入: Option<认知注入>,
    pub 日志记录器: Arc<Mutex<运行日志记录器>>,
    /// 商业级 LLM 池（未配置/未装配时 None，LLM 相关接口返回 未配置）
    pub llm池: Option<Arc<LLM池>>,
    pub 鉴权令牌: Option<String>,
}

impl 数据服务状态 {
    pub fn 新(
        图谱: Arc<Mutex<图谱>>,
        心智地图: Arc<Mutex<心智地图>>,
        语境: Arc<Mutex<过程上下文>>,
        日志记录器: Arc<Mutex<运行日志记录器>>,
        llm池: Option<Arc<LLM池>>,
        鉴权令牌: Option<String>,
    ) -> Self {
        数据服务状态 {
            图谱,
            心智地图,
            语境,
            认知注入: None,
            日志记录器,
            llm池,
            鉴权令牌,
        }
    }
}
