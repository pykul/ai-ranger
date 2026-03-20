export interface OverviewStats {
  total_connections: number;
  active_users: number;
  provider_count: number;
}

export interface ProviderBreakdown {
  provider: string;
  connections: number;
  unique_users: number;
}

export interface UserActivity {
  os_username: string;
  connections: number;
}

export interface MachineActivity {
  machine_hostname: string;
  connections: number;
}

export interface TrafficPoint {
  timestamp: string;
  provider: string;
  connections: number;
}

export interface EventRow {
  timestamp: string;
  os_username: string;
  machine_hostname: string;
  provider: string;
  provider_host: string;
  process_name: string;
  os_type: string;
  detection_method: string;
  src_ip: string;
  model_hint: string;
  process_path: string;
  capture_mode: string;
}

export interface EventsResult {
  events: EventRow[] | null;
  total: number;
  page: number;
  limit: number;
}

export interface FleetAgent {
  ID: string;
  OrgID: string;
  Hostname: string;
  OsUsername: string;
  Os: string;
  AgentVersion: string;
  Status: string;
  EnrolledAt: string;
  LastSeenAt: string | null;
}

export interface OrgSettings {
  org_id: string;
  webhook_url: string | null;
}
