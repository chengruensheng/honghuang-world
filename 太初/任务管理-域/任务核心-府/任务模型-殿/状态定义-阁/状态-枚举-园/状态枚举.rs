use serde::{Deserialize, Serialize};

/// 任务状态：待受理 → 进行中 → 已完成（木之生长），
/// 扩展洪荒五层流转：待受理 → 待圣人设计 → 圣人设计中 → 待大罗金仙实现
/// → 大罗金仙实现中 → 待准圣验收 → 准圣验收中 →（不通过）待修复 → 大罗金仙实现中
/// 或（通过）→ 待道祖终审 → 道祖终审中 →（通过）待人工验收 → 人工验收中
/// →（审核通过）待清理 → 清理中 → 清理完成 /（审核驳回）定向回退 /（终审不通过）待修复
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    // 原有状态（兼容）
    待受理,
    进行中,
    已完成,
    已取消,
    // 新增：洪荒五层流转状态
    待圣人设计,
    圣人设计中,
    待大罗金仙实现,
    大罗金仙实现中,
    待准圣验收,
    准圣验收中,
    待修复,
    待道祖终审,
    道祖终审中,
    // 新增：最终审核阶段（道祖终审后插入，默认 LLM 审核、人工可覆盖）
    待人工验收,
    人工验收中,
    // 新增：太乙金仙清理阶段
    待清理,
    清理中,
    清理完成,
    // 新增：定向回退到木层级（需求偏差/回退次数超限）由道祖澄清后重新进入设计
    待道祖澄清,
    道祖澄清中,
    // 新增：召回状态（触发任务回退后，受影响任务被召回暂停，待触发任务完成后由召回器解除恢复）
    待重新设计,
    待重新实现,
    待重新验收,
    待重新清理,
    // 新增：契约不可满足终态（设计层判定需求在数学/信息论上不可能满足时，如实上报而非降级冒名或空转回退）。
    // 动因：外部标尺实测（2026-09-13）暴露「无解」不是一等公民——同一无解需求，
    // 系统一次「降级实现后自报通过」（造假）、一次「拒绝实现后被验收反复打回至卡死」（缺陷 15-1/15-2）。
    已确认无解,
}

impl TaskStatus {
    /// 合法流转判断（状态机约束）
    pub fn 可流转到(&self, next: &TaskStatus) -> bool {
        matches!(
            (self, next),
            // 原有流转（兼容）
            (TaskStatus::待受理, TaskStatus::进行中)
                | (TaskStatus::待受理, TaskStatus::已取消)
                | (TaskStatus::进行中, TaskStatus::已完成)
                | (TaskStatus::进行中, TaskStatus::已取消)
                // 五层流转
                | (TaskStatus::待受理, TaskStatus::待圣人设计)
                | (TaskStatus::待圣人设计, TaskStatus::圣人设计中)
                | (TaskStatus::圣人设计中, TaskStatus::待大罗金仙实现)
                | (TaskStatus::待大罗金仙实现, TaskStatus::大罗金仙实现中)
                | (TaskStatus::大罗金仙实现中, TaskStatus::待准圣验收)
                | (TaskStatus::待准圣验收, TaskStatus::准圣验收中)
                | (TaskStatus::准圣验收中, TaskStatus::待修复)
                | (TaskStatus::准圣验收中, TaskStatus::待道祖终审)
                | (TaskStatus::待修复, TaskStatus::大罗金仙实现中)
                | (TaskStatus::待道祖终审, TaskStatus::道祖终审中)
                | (TaskStatus::道祖终审中, TaskStatus::待人工验收)
                | (TaskStatus::道祖终审中, TaskStatus::待修复)
                // 最终审核流转（道祖终审后插入；审核驳回先提交到待修复，再定向回退）
                | (TaskStatus::待人工验收, TaskStatus::人工验收中)
                | (TaskStatus::人工验收中, TaskStatus::待清理)
                | (TaskStatus::人工验收中, TaskStatus::待修复)
                // 太乙金仙清理流转
                | (TaskStatus::待清理, TaskStatus::清理中)
                | (TaskStatus::清理中, TaskStatus::清理完成)
                // 道祖澄清流转（定向回退到木层级后）
                | (TaskStatus::待道祖澄清, TaskStatus::道祖澄清中)
                | (TaskStatus::道祖澄清中, TaskStatus::待圣人设计)
                | (TaskStatus::道祖澄清中, TaskStatus::已取消)
                // 契约不可满足终态：设计层判定需求无解时如实上报（含澄清后重新判定仍无解的场景）
                | (TaskStatus::圣人设计中, TaskStatus::已确认无解)
                | (TaskStatus::道祖澄清中, TaskStatus::已确认无解)
        )
    }

    /// 是否终态：走完全流程（清理完成）/ 显式取消（已取消）/ 如实认定无解（已确认无解）/ 兼容旧状态（已完成）。
    /// 终态任务不再被自动驱动承接，并触发终态副作用（清除任务级临时规则）。
    pub fn 是终态(&self) -> bool {
        matches!(
            self,
            TaskStatus::清理完成
                | TaskStatus::已完成
                | TaskStatus::已取消
                | TaskStatus::已确认无解
        )
    }
}
