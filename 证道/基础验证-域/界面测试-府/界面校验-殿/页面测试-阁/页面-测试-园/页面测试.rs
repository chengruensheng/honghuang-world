#[cfg(test)]
mod tests {
    use qk_web::{页面源码, 落盘页面};

    #[test]
    fn 页面含背景令牌与布局槽位() {
        let 页 = 页面源码();
        assert!(页.contains("--契"), "缺少背景层设计令牌");
        assert!(页.contains("data-槽位名=\"主区\""), "缺少布局层槽位");
    }

    #[test]
    fn 页面无未替换占位符() {
        assert!(!页面源码().contains("{{"), "存在未替换占位符");
    }

    #[test]
    fn 页面含槽位挂载契约() {
        assert!(页面源码().contains("乾坤界面"), "缺少槽位挂载契约");
        assert!(页面源码().contains("挂载:"), "挂载接口缺失");
    }

    #[test]
    fn 页面含环境配置与真实对话接入() {
        let 页 = 页面源码();
        assert!(页.contains("window.乾坤配置"), "缺少后端地址环境注入");
        assert!(页.contains("/api/dev/chat"), "对话流未接真实道祖对话接口");
        assert!(页.contains("数据服务未在线") || 页.contains("道祖暂未在线"), "对话缺少可见降级提示");
    }

    #[test]
    fn 页面含真实数据接入() {
        let 页 = 页面源码();
        assert!(页.contains("/api/llm/status"), "设置面板未接 LLM 状态接口");
        assert!(页.contains("/api/llm/select"), "设置面板未接 LLM 切换接口");
        assert!(页.contains("/api/memories"), "左栏视图未接记忆接口");
        assert!(页.contains("/api/cognition/graph"), "左栏视图未接图谱接口");
        assert!(页.contains("视图-抽屉"), "左栏视图抽屉缺失");
    }

    #[test]
    fn 页面含三殿文件面板接入() {
        let 页 = 页面源码();
        assert!(页.contains("/api/files"), "左栏视图未接文件清单接口");
        assert!(页.contains("/api/files/content"), "左栏视图未接文件内容接口");
        assert!(页.contains("视图-阅读"), "阅读覆盖层缺失");
        assert!(页.contains("载文件"), "三殿文件视图缺失");
    }

    #[test]
    fn 页面含任务看板接入() {
        let 页 = 页面源码();
        assert!(页.contains("/api/board"), "看板未接任务接口");
        assert!(页.contains("看板列组"), "看板五行分列缺失");
        assert!(页.contains("主区视图"), "主区视图切换契约缺失");
        assert!(页.contains("乾坤看板"), "看板组件缺失");
        assert!(页.contains("五行映射"), "状态→五行映射缺失");
        assert!(页.contains("/api/dev/pilot"), "看板驱动入口缺失");
        assert!(页.contains("驱动到空闲"), "看板驱动到空闲按钮缺失");
    }

    #[test]
    fn 页面含工作区选择器() {
        let 页 = 页面源码();
        assert!(页.contains("/api/workspace"), "顶栏未接工作区接口");
        assert!(页.contains("顶栏-工作区"), "顶栏工作区选择器缺失");
        assert!(页.contains("乾坤顶栏"), "顶栏组件缺失");
    }

    #[test]
    fn 落盘往返内容一致() {
        let 目录 = std::env::temp_dir()
            .join("qk_web_test_界面")
            .to_string_lossy()
            .into_owned();
        let 路径 = 落盘页面(&目录).unwrap();
        let 内容 = std::fs::read_to_string(&路径).unwrap();
        assert_eq!(内容, 页面源码(), "落盘内容与组装页面不一致");
        std::fs::remove_dir_all(&目录).ok();
    }
}
