import { writable } from "svelte/store";

export interface Device {
  instance: number;
  name: string;
  vendor_id: number;
  vendor_name: string;
}

export interface BacnetObject {
  object_type: string;
  instance: number;
  name: string;
}

export interface Property {
  id: string;
  name: string;
  value: string;
  data_type: string;
  writable: boolean;
  highlight_opacity: number;
}

export interface DeviceInfo {
  vendor_name: string;
  model_name: string;
  firmware_revision: string;
  app_software_version: string;
  protocol_version: number;
  protocol_revision: number;
  description: string;
  location: string;
  database_revision: number;
  max_apdu_length: number;
  apdu_timeout: number;
  apdu_segment_timeout: number;
}

export interface NetworkStats {
  packets_sent: number;
  packets_received: number;
  bytes_sent: number;
  bytes_received: number;
  errors: number;
  avg_response_time_ms: number;
}

export const selectedDevice = writable<Device | null>(null);
export const selectedObject = writable<BacnetObject | null>(null);
export const objects = writable<BacnetObject[]>([]);
export const properties = writable<Property[]>([]);

export interface ComparisonItem {
  device_id: number;
  object_type: string;
  instance: number;
}

export interface ComparisonObject {
  device_id: number;
  device_name: string;
  object_type: string;
  instance: number;
  object_name: string;
}

export interface ComparisonProperty {
  property_name: string;
  values: (string | null)[];
}

export interface ComparisonResult {
  objects: ComparisonObject[];
  properties: ComparisonProperty[];
}

export interface DeviceHealth {
  is_online: boolean;
  consecutive_failures: number;
  max_consecutive_failures: number;
  last_success: number | null;
  last_failure: number | null;
}

export const deviceHealth = writable<Record<number, DeviceHealth>>({});

export const comparisonItems = writable<ComparisonItem[]>([]);
export const comparisonResult = writable<ComparisonResult | null>(null);
export const showComparison = writable<boolean>(false);
