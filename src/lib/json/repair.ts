import { jsonrepair } from "jsonrepair";

export function repairJson(text: string): string {
  return jsonrepair(text);
}
