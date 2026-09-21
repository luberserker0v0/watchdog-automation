import type { NodeMetaInfo, NodeParamValue } from "../../../../api/nodeTest";

export function defaultNodeParams(node: Pick<NodeMetaInfo, "params">): Record<string, NodeParamValue> {
  return Object.fromEntries(
    node.params
      .filter((param) => param.default !== undefined && param.default !== null)
      .map((param) => [param.key, param.default as NodeParamValue]),
  );
}
