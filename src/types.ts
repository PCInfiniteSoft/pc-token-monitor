export interface WindowUsage {
  utilization: number;   // 0.0 – 1.0
  resets_at: string;     // ISO 8601
}

export type DataSource = "oauth" | "jsonl_fallback";

export interface UsageData {
  five_hour: WindowUsage;
  seven_day: WindowUsage;
  seven_day_opus_utilization: number | null;
  extra_usage_enabled: boolean;
  source: DataSource;
}

export type Plan = "Pro" | "Max50" | "Max200" | "Unknown";

export interface AppConfig {
  plan: Plan;
}

export interface FrontendState {
  usage: UsageData | null;
  config: AppConfig;
}
