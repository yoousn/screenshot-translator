export type BasicReadinessStep = {
  id: string;
  ready: boolean;
  severity?: "success" | "warning" | "error" | string;
  label?: string;
  description?: string;
  nextAction?: string;
};

type ReadinessTextLabels = Record<string, string>;

type DisplayReadinessStep<T extends BasicReadinessStep> = T & {
  displayLabel: string;
  displayDescription: string;
  displayNextAction: string;
};

const keyOf = (prefix: string, value?: string) => {
  if (!value) return "";
  return prefix + value.split("-").map((part) => part.charAt(0).toUpperCase() + part.slice(1)).join("");
};

export const localizeReadinessStep = <T extends BasicReadinessStep>(step: T, labels: ReadinessTextLabels): DisplayReadinessStep<T> => {
  const label = labels[keyOf("readinessStep", step.id)] || step.label || step.id;
  const description = labels[keyOf("readinessStepDesc", step.id)] || step.description || "";
  const nextAction = labels[keyOf("readinessAction", step.nextAction)] || step.nextAction || "";
  return {
    ...step,
    displayLabel: label,
    displayDescription: description,
    displayNextAction: nextAction,
  };
};

export const localizeOcrReadinessStep = localizeReadinessStep;
