#[path = "流式解析.rs"]
pub mod 流式解析;
pub use 流式解析::*;

#[cfg(test)]
mod 测试 {
    use super::*;

    fn 转词(们: &[&str]) -> Vec<String> {
        们.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn 空输入_返回空调用() {
        let 调用 = 解析调用(vec![]);
        assert_eq!(调用.域, "");
        assert_eq!(调用.动作, "");
        assert!(调用.参数.is_empty());
        assert!(调用.旗标.is_empty());
        assert!(!调用.要JSON);
    }

    #[test]
    fn 调用_空_构造空字段() {
        let 调用 = 调用::空();
        assert_eq!(调用.域, "");
        assert_eq!(调用.动作, "");
        assert!(调用.参数.is_empty());
        assert!(调用.旗标.is_empty());
        assert!(!调用.要JSON);
    }

    #[test]
    fn 仅域_填充域字段() {
        let 调用 = 解析调用(转词(&["世界"]));
        assert_eq!(调用.域, "世界");
        assert_eq!(调用.动作, "");
        assert!(调用.参数.is_empty());
    }

    #[test]
    fn 域加动作_两段填充() {
        let 调用 = 解析调用(转词(&["世界", "守护"]));
        assert_eq!(调用.域, "世界");
        assert_eq!(调用.动作, "守护");
        assert!(调用.参数.is_empty());
    }

    #[test]
    fn 域动作加单参数() {
        let 调用 = 解析调用(转词(&["想法", "投递", "第一段文本"]));
        assert_eq!(调用.域, "想法");
        assert_eq!(调用.动作, "投递");
        assert_eq!(调用.参数, vec!["第一段文本".to_string()]);
    }

    #[test]
    fn 域动作加多参数_顺序保持() {
        let 调用 = 解析调用(转词(&["流水", "跟踪", "流水id", "额外a", "额外b"]));
        assert_eq!(调用.域, "流水");
        assert_eq!(调用.动作, "跟踪");
        assert_eq!(
            调用.参数,
            vec![
                "流水id".to_string(),
                "额外a".to_string(),
                "额外b".to_string()
            ]
        );
    }

    #[test]
    fn json_旗标_置要json() {
        let 调用 = 解析调用(转词(&["想法", "投递", "文本", "--json"]));
        assert!(调用.要JSON);
        assert_eq!(调用.参数, vec!["文本".to_string()]);
    }

    #[test]
    fn 全文_旗标_落键值对() {
        let 调用 = 解析调用(转词(&["流水", "跟踪", "流水id", "--全文"]));
        assert_eq!(调用.旗标, vec![("全文".to_string(), "true".to_string())]);
    }

    #[test]
    fn 短旗标_t_后跟令牌值() {
        let 调用 = 解析调用(转词(&["想法", "投递", "-t", "mytoken", "文本"]));
        assert_eq!(调用.旗标, vec![("令牌".to_string(), "mytoken".to_string())]);
        assert_eq!(调用.参数, vec!["文本".to_string()]);
    }

    #[test]
    fn 长旗标_令牌_后跟令牌值() {
        let 调用 = 解析调用(转词(&["想法", "投递", "--令牌", "mytoken", "文本"]));
        assert_eq!(调用.旗标, vec![("令牌".to_string(), "mytoken".to_string())]);
        assert_eq!(调用.参数, vec!["文本".to_string()]);
    }

    #[test]
    fn 短旗标_f_后跟文件值() {
        let 调用 = 解析调用(转词(&["设计", "上呈", "-f", "路径/设计.md", "内容"]));
        assert_eq!(
            调用.旗标,
            vec![("文件".to_string(), "路径/设计.md".to_string())]
        );
        assert_eq!(调用.参数, vec!["内容".to_string()]);
    }

    #[test]
    fn 长旗标_文件_后跟文件值() {
        let 调用 = 解析调用(转词(&["设计", "上呈", "--文件", "路径/设计.md", "内容"]));
        assert_eq!(
            调用.旗标,
            vec![("文件".to_string(), "路径/设计.md".to_string())]
        );
        assert_eq!(调用.参数, vec!["内容".to_string()]);
    }

    #[test]
    fn 短旗标_意见_后跟意见值() {
        let 调用 = 解析调用(转词(&[
            "验收",
            "裁决",
            "要求id",
            "通过",
            "-意见",
            "全部合格",
        ]));
        assert_eq!(
            调用.旗标,
            vec![("意见".to_string(), "全部合格".to_string())]
        );
        assert_eq!(调用.参数, vec!["要求id".to_string(), "通过".to_string()]);
    }

    #[test]
    fn 长旗标_意见_后跟意见值() {
        let 调用 = 解析调用(转词(&["验收", "裁决", "要求id", "--意见", "补充"]));
        assert_eq!(调用.旗标, vec![("意见".to_string(), "补充".to_string())]);
        assert_eq!(调用.参数, vec!["要求id".to_string()]);
    }

    #[test]
    fn 旗标缺值_落空字符串且不消耗下个词() {
        // -t 置于末尾且后无词：旗标值=空串；其之前的裸词仍按段位进入参数。
        let 调用 = 解析调用(转词(&["想法", "投递", "后置参数", "-t"]));
        assert_eq!(调用.旗标, vec![("令牌".to_string(), "".to_string())]);
        assert_eq!(调用.参数, vec!["后置参数".to_string()]);
    }

    #[test]
    fn 旗标在中段_消耗紧邻词作为旗标值() {
        // 实现契约：旗标分支内联取迭代器下一项作旗标值，紧邻词不再作为参数。
        let 调用 = 解析调用(转词(&["想法", "投递", "-t", "tok值"]));
        assert_eq!(调用.旗标, vec![("令牌".to_string(), "tok值".to_string())]);
        assert!(调用.参数.is_empty());
    }

    #[test]
    fn 旗标与json混合_互不干扰() {
        let 调用 = 解析调用(转词(&[
            "世界", "守护", "--json", "-t", "tok", "--全文", "后置",
        ]));
        assert!(调用.要JSON);
        assert_eq!(调用.旗标.len(), 2);
        assert!(调用.旗标.contains(&("令牌".to_string(), "tok".to_string())));
        assert!(调用
            .旗标
            .contains(&("全文".to_string(), "true".to_string())));
        assert_eq!(调用.参数, vec!["后置".to_string()]);
    }

    #[test]
    fn 重复首词_第二词被认作动作() {
        // 第一段位被填后，第二段位由后续裸词填充（实现按"首个裸词=域、次个裸词=动作"）。
        let 调用 = 解析调用(转词(&["世界", "守护", "v1"]));
        assert_eq!(调用.域, "世界");
        assert_eq!(调用.动作, "守护");
        assert_eq!(调用.参数, vec!["v1".to_string()]);
    }

    #[test]
    fn 长中文与特殊字符_原样保留() {
        // 解析器不做语义清洗：长中文、Unicode、含旗标前缀短语的参数原样落到参数。
        let 调用 = 解析调用(转词(&[
            "想法",
            "投递",
            "这是一段含【标点】与 emoji 风格与 --看似旗标 的文本",
        ]));
        assert_eq!(调用.域, "想法");
        assert_eq!(调用.动作, "投递");
        assert_eq!(调用.参数.len(), 1);
        assert!(调用.旗标.is_empty());
        assert!(调用.参数[0].contains("--看似旗标"));
    }

    #[test]
    fn 仅旗标无域动作_旗标仍生效() {
        // 边界：纯旗标序列，域/动作为空，旗标正常落定。
        let 调用 = 解析调用(转词(&["--json", "-t", "tok"]));
        assert_eq!(调用.域, "");
        assert_eq!(调用.动作, "");
        assert!(调用.要JSON);
        assert_eq!(调用.旗标, vec![("令牌".to_string(), "tok".to_string())]);
    }

    #[test]
    fn 空字符串token_作为域() {
        // 边界：单个空字符串 token，首个段位=域、动作留空、参数空。
        let 调用 = 解析调用(转词(&[""]));
        assert_eq!(调用.域, "");
        assert_eq!(调用.动作, "");
        assert!(调用.参数.is_empty());
        assert!(调用.旗标.is_empty());
        assert!(!调用.要JSON);
    }

    #[test]
    fn 重复json旗标_幂等() {
        // 边界：多次 --json 重复，布尔不变（无累加效应）。
        let 调用 = 解析调用(转词(&["--json", "世界", "守护", "--json", "--json"]));
        assert!(调用.要JSON);
        assert_eq!(调用.域, "世界");
        assert_eq!(调用.动作, "守护");
        assert!(调用.参数.is_empty());
    }

    #[test]
    fn 旗标后紧接旗标名_旗标名被吞为旗标值() {
        // 边界：旗标分支无条件消耗下一项作值，即便下一项是另一个旗标名也不识别为旗标。
        let 调用 = 解析调用(转词(&["-t", "--json"]));
        assert_eq!(调用.旗标, vec![("令牌".to_string(), "--json".to_string())]);
        assert!(!调用.要JSON);
        assert_eq!(调用.域, "");
        assert!(调用.参数.is_empty());
    }

    #[test]
    fn 旗标吞旗标名后_剩余token按段位填充() {
        // 边界：-t 吞 "--json" 作令牌值后，"tok" 进入段位填充（首个裸词=域）。
        let 调用 = 解析调用(转词(&["-t", "--json", "tok"]));
        assert_eq!(调用.旗标, vec![("令牌".to_string(), "--json".to_string())]);
        assert!(!调用.要JSON);
        assert_eq!(调用.域, "tok");
        assert!(调用.参数.is_empty());
    }

    #[test]
    fn 未知短旗标_落入段位() {
        // 边界：未知短旗标如 -x 不被识别，按段位填充（首个=域，次个=动作）。
        let 调用 = 解析调用(转词(&["-x", "世界"]));
        assert_eq!(调用.域, "-x");
        assert_eq!(调用.动作, "世界");
        assert!(调用.旗标.is_empty());
        assert!(!调用.要JSON);
    }

    #[test]
    fn 未知长旗标_落入段位() {
        // 边界：未知长旗标如 --未知旗标 不识别为旗标，按段位与参数填充。
        let 调用 = 解析调用(转词(&["--未知旗标", "世界", "守护"]));
        assert_eq!(调用.域, "--未知旗标");
        assert_eq!(调用.动作, "世界");
        assert_eq!(调用.参数, vec!["守护".to_string()]);
        assert!(调用.旗标.is_empty());
    }

    #[test]
    fn 旗标名大小写不匹配_落入段位() {
        // 边界：旗标识别大小写敏感，--JSON / --Token 不识别。
        let 调用 = 解析调用(转词(&["--JSON", "世界", "守护"]));
        assert_eq!(调用.域, "--JSON");
        assert_eq!(调用.动作, "世界");
        assert_eq!(调用.参数, vec!["守护".to_string()]);
        assert!(调用.旗标.is_empty());
        assert!(!调用.要JSON);
    }

    #[test]
    fn 多旗标累积_顺序保持() {
        // 边界：多旗标按出现顺序累积到旗标 vec，不被打乱。
        let 调用 = 解析调用(转词(&["-t", "tok1", "-f", "path1", "-意见", "note1"]));
        assert_eq!(
            调用.旗标,
            vec![
                ("令牌".to_string(), "tok1".to_string()),
                ("文件".to_string(), "path1".to_string()),
                ("意见".to_string(), "note1".to_string()),
            ]
        );
    }

    #[test]
    fn 调用_克隆_字段全等() {
        // 边界：#[derive(Clone)] 后所有字段深拷贝相等。
        let 原 = 解析调用(转词(&["世界", "守护", "arg1", "--json", "-t", "tok"]));
        let 克隆 = 原.clone();
        assert_eq!(原.域, 克隆.域);
        assert_eq!(原.动作, 克隆.动作);
        assert_eq!(原.参数, 克隆.参数);
        assert_eq!(原.旗标, 克隆.旗标);
        assert_eq!(原.要JSON, 克隆.要JSON);
    }

    #[test]
    fn 调用_调试_不panic() {
        // 边界：#[derive(Debug)] 格式化对 emoji/中文参数不 panic 且包含原文。
        let 调用 = 解析调用(转词(&["世界", "守护", "arg含emoji🤖"]));
        let s = format!("{:?}", 调用);
        assert!(s.contains("世界"));
        assert!(s.contains("守护"));
        assert!(s.contains("arg含emoji🤖"));
    }
}
