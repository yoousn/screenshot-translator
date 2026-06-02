import { Alert, Button, Card, List, Space, Tag, Typography } from "antd";
import { CheckCircleOutlined, ExclamationCircleOutlined, ReloadOutlined, ToolOutlined } from "@ant-design/icons";
import type { DiagnosticsIssue, DiagnosticsReport } from "../../hooks/useDiagnosticsReport";
import { localizeReadinessStep } from "../config/ocrReadinessStepText";

const { Text } = Typography;

type DashboardDiagnosticsLabels = Record<string, string> & {
  diagnosticsTitle: string;
  diagnosticsDesc: string;
  diagnosticsRefresh: string;
  diagnosticsReady: string;
  diagnosticsNotReady: string;
  diagnosticsOpenRecovery: string;
  diagnosticsNoIssues: string;
  diagnosticsIssuesCount: string;
};

type DashboardDiagnosticsCardProps = {
  labels: DashboardDiagnosticsLabels;
  report: DiagnosticsReport | null;
  loading: boolean;
  error?: string | null;
  onRefresh: () => void;
  onOpenModels: () => void;
  onOpenSettings: () => void;
};

const issueColor = (issue: DiagnosticsIssue) => {
  if (issue.severity === "error") return "red";
  if (issue.severity === "warning") return "orange";
  return "blue";
};

const moduleTarget = (module: string) => (module === "ocrRuntime" || module === "recording" ? "models" : "settings");
const moduleLabel = (module: string, labels: DashboardDiagnosticsLabels) => labels[`diagnosticsModule${module.charAt(0).toUpperCase()}${module.slice(1)}`] || module;

export default function DashboardDiagnosticsCard({ labels, report, loading, error, onRefresh, onOpenModels, onOpenSettings }: DashboardDiagnosticsCardProps) {
  const health = report?.health;
  const issues = health?.issues || [];
  const visibleIssues = issues.slice(0, 4);
  const ready = Boolean(health?.ready);

  const openRecovery = (module: string) => {
    if (moduleTarget(module) === "models") onOpenModels();
    else onOpenSettings();
  };

  return (
    <Card bordered={false} style={{ borderRadius: 18, boxShadow: "0 18px 48px rgba(15,23,42,0.08)" }}>
      <Space direction="vertical" size={14} style={{ width: "100%" }}>
        <Space align="start" style={{ justifyContent: "space-between", width: "100%" }}>
          <Space align="start">
            <div style={{ width: 40, height: 40, borderRadius: 14, background: "linear-gradient(135deg, #fee2e2 0%, #fef3c7 100%)", display: "flex", alignItems: "center", justifyContent: "center", color: ready ? "#16a34a" : "#dc2626", fontSize: 18 }}>
              {ready ? <CheckCircleOutlined /> : <ToolOutlined />}
            </div>
            <div>
              <Text strong style={{ display: "block", color: "#0f172a", fontSize: 15 }}>{labels.diagnosticsTitle}</Text>
              <Text type="secondary" style={{ fontSize: 12 }}>{labels.diagnosticsDesc}</Text>
            </div>
          </Space>
          <Space>
            {health && <Tag color={ready ? "success" : "error"} icon={ready ? <CheckCircleOutlined /> : <ExclamationCircleOutlined />} style={{ borderRadius: 999, margin: 0 }}>{ready ? labels.diagnosticsReady : labels.diagnosticsNotReady}</Tag>}
            <Button size="small" icon={<ReloadOutlined />} loading={loading} onClick={onRefresh}>{labels.diagnosticsRefresh}</Button>
          </Space>
        </Space>

        {error && <Alert type="error" showIcon message={error} />}

        {!error && health && (
          <Space direction="vertical" size={10} style={{ width: "100%" }}>
            <Space wrap>
              <Tag color={ready ? "green" : "red"}>{labels.diagnosticsIssuesCount.replace("{count}", String(health.issueCount || 0))}</Tag>
              {Object.entries(health.issuesByModule || {}).map(([module, count]) => (
                <Tag key={module} color="orange">{module}: {count}</Tag>
              ))}
            </Space>
            {health.readinessByModule && (
              <Space direction="vertical" size={4} style={{ width: "100%" }}>
                {Object.entries(health.readinessByModule).map(([module, readiness]) => {
                  const firstBlocked = readiness.firstBlockedStep;
                  const displayBlocked = firstBlocked?.id
                    ? localizeReadinessStep({ ready: false, severity: "warning", label: firstBlocked.label || firstBlocked.id, description: firstBlocked.description || "", id: firstBlocked.id, nextAction: firstBlocked.nextAction || "" }, labels)
                    : null;
                  return (
                    <Text key={module} type="secondary" style={{ fontSize: 12 }}>
                      {moduleLabel(module, labels)}: {readiness.readySteps}/{readiness.totalSteps} · {readiness.ready ? labels.diagnosticsReady : `${displayBlocked?.displayLabel || firstBlocked?.label || firstBlocked?.id || labels.diagnosticsNotReady} → ${displayBlocked?.displayNextAction || firstBlocked?.nextAction || labels.diagnosticsOpenRecovery}`}
                    </Text>
                  );
                })}
              </Space>
            )}

            {visibleIssues.length === 0 ? (
              <Alert type="success" showIcon message={labels.diagnosticsNoIssues} />
            ) : (
              <List
                size="small"
                dataSource={visibleIssues}
                renderItem={(issue) => (
                  <List.Item actions={[<Button key="open" size="small" onClick={() => openRecovery(issue.module)}>{labels.diagnosticsOpenRecovery}</Button>]}> 
                    <List.Item.Meta
                      title={<Space wrap><Tag color={issueColor(issue)}>{issue.severity}</Tag><Text strong>{issue.code}</Text><Tag>{issue.module}</Tag></Space>}
                      description={<Space direction="vertical" size={2}><Text>{issue.message}</Text><Text type="secondary">{issue.nextAction}</Text></Space>}
                    />
                  </List.Item>
                )}
              />
            )}
          </Space>
        )}
      </Space>
    </Card>
  );
}


