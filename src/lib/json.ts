export interface JsonValidation {
  valid: boolean;
  message: string;
}

export function validateJson(input: string): JsonValidation {
  if (!input.trim()) {
    return { valid: false, message: "JSON 不能为空" };
  }

  try {
    JSON.parse(input);
    return { valid: true, message: "JSON 有效" };
  } catch (error) {
    return {
      valid: false,
      message: error instanceof Error ? error.message : "JSON 格式错误"
    };
  }
}

export function validateJsonTemplate(input: string): JsonValidation {
  const preview = input
    .replace(/\{\{jsonMessage\}\}/g, '{"hello":"world"}')
    .replace(/\{\{message\}\}/g, "sample message")
    .replace(/\{\{peerId\}\}/g, "preview-peer")
    .replace(/\{\{requestId\}\}/g, "preview-request")
    .replace(/\{\{timestamp\}\}/g, "2026-05-12T00:00:00Z");

  return validateJson(preview);
}

export function formatJson(input: string): string {
  return JSON.stringify(JSON.parse(input), null, 2);
}

export function compactJson(input: string): string {
  return JSON.stringify(JSON.parse(input));
}

export function safeFormatTimestamp(input: string): string {
  const date = new Date(input);
  if (Number.isNaN(date.getTime())) {
    return input;
  }

  return new Intl.DateTimeFormat("zh-CN", {
    hour12: false,
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit"
  }).format(date);
}

export function makeEndpoint(host: string, port: number, path: string): string {
  const normalizedPath = path.startsWith("/") ? path : `/${path}`;
  return `ws://${host}:${port}${normalizedPath || "/"}`;
}
