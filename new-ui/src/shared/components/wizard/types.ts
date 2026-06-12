export interface WizardPageConfig {
  title: string;
  subtitle: string;
  activeStep: string;
  steps: Record<string, WizardPageStep>;
}

export interface WizardDocsLink {
  link: string;
  label: string;
}

export interface WizardPageStep {
  id: string;
  order: number;
  label: string;
  description?: string;
  hidden?: boolean;
}
