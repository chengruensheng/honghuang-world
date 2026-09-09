/* ═══════════════════════════════════════════════════════════
   乾坤 · 槽位挂载契约 —— 布局与组件之间的唯一接口
   组件只允许经 乾坤界面.挂载(槽位名, 节点) 进入布局；
   禁止组件直接改写 .界面-* 结构，以此保证布局与组件隔离。
   ═══════════════════════════════════════════════════════════ */
window.乾坤界面 = (function(){
  var 槽位表 = {};
  document.querySelectorAll("[data-槽位名]").forEach(function(槽){
    槽位表[槽.dataset.槽位名] = 槽;
  });

  return {
    /** 只读槽位表（调试用） */
    槽位表: 槽位表,

    /**
     * 挂载组件到槽位
     * @param {string} 名 - 槽位名（顶栏/左栏/主区/右栏/底栏）
     * @param {Node|Function} 节点 - DOM 节点，或返回节点的工厂函数
     * @returns {Node|null} 实际挂载的节点；未知槽位返回 null
     */
    挂载: function(名, 节点){
      var 槽 = 槽位表[名];
      if (!槽){ console.warn("[乾坤界面] 未知槽位：" + 名); return null; }
      var 节 = typeof 节点 === "function" ? 节点() : 节点;
      if (!节){ return null; }
      槽.appendChild(节);
      return 节;
    }
  };
})();
