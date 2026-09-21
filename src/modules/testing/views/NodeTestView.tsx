import NodeTester from "../../pipeline/authoring/components/NodeTester";

export default function NodeTestView() {
  return (
    <div className="node-test-view">
      <div className="node-test-header">
        <h2>Node-Test</h2>
      </div>
      <div className="node-test-body">
        <NodeTester />
      </div>
    </div>
  );
}
