import NodeTester from "../components/NodeTester";
import { useTranslation } from "react-i18next";

export default function NodeTestView() {
  const { t } = useTranslation();
  return (
    <div className="node-test-view">
      <div className="node-test-header">
        <h2>{t("nav.nodeTest")}</h2>
      </div>
      <div className="node-test-body">
        <NodeTester />
      </div>
    </div>
  );
}
