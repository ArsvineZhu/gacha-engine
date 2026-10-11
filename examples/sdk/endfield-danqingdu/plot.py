"""将丹青渡卡池测试结果绘制为概率分布图，并生成中文报告。"""
import json
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib.ticker import PercentFormatter

ROOT = Path(__file__).resolve().parents[3]
OUTPUT = ROOT / "examples/sdk/endfield-danqingdu/reports"
COLORS = {"fixed-five": "#2457A6", "proportional": "#B96A18", "weapon": "#177864"}
LABELS = {"fixed-five": "保留六星概率", "proportional": "条件化五星以上（对照）", "weapon": "武库批次模型"}


def read(slug, model):
    name = f"{slug}-{model}"
    report = json.loads((OUTPUT / f"{name}.json").read_text(encoding="utf-8"))
    metadata = next(c for c in json.loads((ROOT / "examples/sdk/endfield-danqingdu/cases.json").read_text(encoding="utf-8")) if c["case"] == name)
    return {"report": report, "metadata": metadata}


def cumulative(case, batches=False):
    analysis = case["report"]["analysis"]
    points = analysis["action_distribution"] if batches else analysis["draw_distribution"]["distribution"]
    key = "action" if batches else "draw"
    return [p[key] for p in points], [p["cumulative_probability"] for p in points]


def draw_chart():
    plt.rcParams.update({"font.family": "sans-serif", "font.sans-serif": ["Microsoft YaHei", "SimHei", "Noto Sans CJK SC", "DejaVu Sans"],
                         "axes.unicode_minus": False, "font.size": 11, "axes.spines.top": False,
                         "axes.spines.right": False, "figure.facecolor": "white"})
    fig, axes = plt.subplots(2, 2, figsize=(14, 9))
    for ax in axes.flat:
        ax.grid(axis="y", alpha=0.2)
        ax.yaxis.set_major_formatter(PercentFormatter(1.0))
    for model in ["fixed-five", "proportional"]:
        case = read("si", model)
        points = case["report"]["analysis"]["draw_distribution"]["distribution"]
        x, y = cumulative(case)
        axes[0, 0].plot(x, y, color=COLORS[model], label=LABELS[model], linewidth=2)
        axes[0, 1].plot(x, [p["first_hit_probability"] for p in points], color=COLORS[model], label=LABELS[model], linewidth=1.7)
        sample = case["report"]["simulation"]["distribution"][9::10]
        axes[0, 0].scatter([p["draw"] for p in sample], [p["cumulative_probability"] for p in sample],
                           color=COLORS[model], marker="x", s=25)
    axes[0, 0].set(title="祀 / 明河：首次获得累计概率 CDF", xlabel="普通寻访次数", ylabel="至少获得一次目标的概率", ylim=(0, 1.04))
    axes[0, 1].set(title="祀 / 明河：首次获得所在抽数 PMF", xlabel="普通寻访次数", ylabel="恰在这一抽首次获得的概率")
    for ax in axes[0]:
        ax.set_xlim(1, 120)
        ax.axvline(65, color="#9AA3AF", linestyle=":", linewidth=1)
        ax.axvline(120, color="#9AA3AF", linestyle=":", linewidth=1)
    axes[0, 0].legend(loc="upper left", frameon=False)
    weapon = read("yuzhongshen", "weapon")
    x, y = cumulative(weapon, True)
    axes[1, 0].plot(x, y, color=COLORS["weapon"], marker="o", linewidth=2, label="逐抽枚举汇总至申领边界")
    points = weapon["report"]["simulation"]["distribution"][9::10]
    axes[1, 0].scatter(x, [p["cumulative_probability"] for p in points], marker="x", color="#27354A", s=35, label="10,000 次独立模拟")
    pmf = [p["first_hit_probability"] for p in weapon["report"]["analysis"]["action_distribution"]]
    axes[1, 1].bar(x, pmf, width=0.65, color=COLORS["weapon"], alpha=0.9)
    axes[1, 0].set(title="玉中身 / 唤月：首次获得累计概率 CDF", xlabel="武库申领次数（每次 10 件）", ylabel="至少获得一次目标的概率", ylim=(0, 1.04))
    axes[1, 1].set(title="玉中身 / 唤月：首次获得所在批次 PMF", xlabel="武库申领次数（每次 10 件）", ylabel="恰在这一批首次获得的概率")
    for ax in axes[1]:
        ax.set_xticks(range(1, 9))
    axes[1, 0].legend(loc="upper left", frameon=False)
    fig.suptitle("终末地「丹青渡」前瞻卡池测试", fontsize=21, fontweight="bold", y=0.98)
    fig.text(0.5, 0.935, "零初始保底与进度 · 普通寻访不执行免费十连 · 目标身份已官宣，概率沿用现有规则模型", ha="center", color="#4B5563")
    fig.text(0.5, 0.025, "同类目标采用相同机制，因此理论分布一致；曲线为模型枚举结果，叉号为随机模拟。非目标名单为参考名单。", ha="center", color="#4B5563", fontsize=10)
    fig.tight_layout(rect=(0, 0.05, 1, 0.91), h_pad=2.5)
    fig.savefig(OUTPUT / "distribution.png", dpi=180)
    plt.close(fig)


def percent(value):
    return f"{value * 100:.3f}%"


def write_report():
    lines = ["# 丹青渡独立接入与概率报告", "",
        "此案例作为独立开发者项目消费公开 SDK。接入方使用独立的官方游戏资料库，通过第一方接口完成概率分析、模拟、trace、会话续跑和 JSON/CSV 导出。", "",
        "角色与武器身份依据[官方导览](https://endfield.hypergryph.com/version_briefing/latest)，版本日期依据[研发通讯](https://endfield.hypergryph.com/news/3805)。核对资料见[摘要](../../../official/gacha-games/tools/reviews/danqingdu-20261011.json)。", "",
        "## 接入口径", "",
        "- 独立项目：[接入示例](README.md)，具有自己的工作区、锁文件、场景配置，卡池资料由官方库提供。",
        "- 接入代码依赖公开 SDK 与官方资料库，不实现抽样、分布计算、动作汇总或统计对照。",
        "- `tools/check_examples.py` 将项目复制到仓库外运行，六个场景各模拟 10,000 个独立初始账户；再在相同独立目录调用公开 CLI 核对分析、trace 与续跑。",
        "- 普通寻访从零保底和零进度开始，不执行免费十连，不计阶段信物；武器按完整申领统计，不计阶段赠礼。",
        "- 完整新版本陪跑名单尚未核实，数据包沿用参考非目标名单，仅用于当期目标的首次获得概率。两种角色模型及武器末抽补位假设随 SDK 报告一起输出。", "",
        "## 概率分布", "", "![分布图](reports/distribution.png)", "",
        "## SDK 返回的期望与分位数", "", "| 目标 | 模型 | 期望完整动作 | 50% | 90% | 95% | 99% | 100% |", "| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |"]
    names = [("si", "fixed-five"), ("si", "proportional"), ("minghe", "fixed-five"), ("minghe", "proportional"), ("yuzhongshen", "weapon"), ("huanyue", "weapon")]
    for slug, model in names:
        case = read(slug, model)
        a = case["report"]["analysis"]
        q = " | ".join(str(p["action"]) for p in a["action_quantiles"])
        lines.append(f"| {case['metadata']['label']} | {LABELS[model]} | {a['expected_actions_if_hit']:.3f} | {q} |")
    lines += ["", "角色单位为普通寻访次数，武器单位为完整申领次数。同类目标使用相同机制与起点，理论分布一致；各场景独立运行。", "",
              "## 完整动作累计概率", "", "| 申领次数 | 玉中身 / 唤月 CDF | 首次获得该批 PMF |", "| ---: | ---: | ---: |"]
    for point in read("yuzhongshen", "weapon")["report"]["analysis"]["action_distribution"]:
        lines.append(f"| {point['action']} | {percent(point['cumulative_probability'])} | {percent(point['first_hit_probability'])} |")
    lines += ["", "## 复现", "", "在仓库根目录：", "", "```sh", "cargo build -p gacha-cli --release --locked", "python tools/check_examples.py", "python examples/sdk/endfield-danqingdu/plot.py", "```", "",
              "也可进入示例目录执行 `cargo run --release --locked -- 10000`。图表仅读取 SDK 返回的分布与汇总，不执行概率算法。", "",
              "[独立运行证据](reports/verification.json)记录工作区隔离、源文件未变化、六个场景及 CLI/SDK 一致性。性能与理论依据见[算法说明](../../../docs/PERFORMANCE.md)，SDK 契约见[接入文档](../../../docs/SDK.md)。", "",
              "## 原始输出", "", "| 场景 | SDK 报告 | CSV | Trace |", "| --- | --- | --- | --- |"]
    for slug, model in names:
        name = f"{slug}-{model}"
        label = read(slug, model)["metadata"]["label"]
        lines.append(f"| {label} / {LABELS[model]} | [报告](reports/{name}.json) | [分布](reports/{name}.csv) | [追踪](reports/{name}.trace.json) |")
    (ROOT / "examples/sdk/endfield-danqingdu/REPORT.md").write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")


if __name__ == "__main__":
    draw_chart()
    write_report()
