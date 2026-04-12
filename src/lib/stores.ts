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

export const selectedDevice = writable<Device | null>(null);
export const selectedObject = writable<BacnetObject | null>(null);
export const objects = writable<BacnetObject[]>([]);
export const properties = writable<Property[]>([]);
