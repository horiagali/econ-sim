import type { ContributionNode } from "../api";

// "Why did this change?" — nested contribution tree (ADR-0008). Children of
// every node sum to the node's value; hover/expand to drill down.
export function WhyTree({ node, depth = 0 }: { node: ContributionNode; depth?: number }) {
  const color = node.value >= 0 ? "#b33" : "#2a7";
  return (
    <details open={depth === 0} style={{ marginLeft: depth * 14 }}>
      <summary>
        <span style={{ color, fontVariantNumeric: "tabular-nums" }}>
          {node.value >= 0 ? "+" : ""}
          {node.value.toFixed(2)}
        </span>{" "}
        {node.label}
      </summary>
      {node.children.map((c, i) => (
        <WhyTree key={i} node={c} depth={depth + 1} />
      ))}
    </details>
  );
}
