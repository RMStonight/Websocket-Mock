import { Check, Code2, WandSparkles, X } from "lucide-react";
import { formatJson, validateJson, validateJsonTemplate } from "../lib/json";

interface JsonEditorProps {
  label: string;
  value: string;
  onChange: (value: string) => void;
  rows?: number;
  template?: boolean;
  disabled?: boolean;
}

export function JsonEditor({
  label,
  value,
  onChange,
  rows = 9,
  template = false,
  disabled = false
}: JsonEditorProps) {
  const validation = template ? validateJsonTemplate(value) : validateJson(value);
  const canFormat = validateJson(value).valid;

  function handleFormat() {
    if (canFormat) {
      onChange(formatJson(value));
    }
  }

  return (
    <label className="json-editor">
      <span className="field-header">
        <span className="field-title">
          <Code2 aria-hidden="true" size={15} />
          {label}
        </span>
        <span className={`validation ${validation.valid ? "is-valid" : "is-invalid"}`}>
          {validation.valid ? <Check aria-hidden="true" size={14} /> : <X aria-hidden="true" size={14} />}
          {validation.message}
        </span>
      </span>
      <textarea
        value={value}
        rows={rows}
        spellCheck={false}
        disabled={disabled}
        onChange={(event) => onChange(event.target.value)}
      />
      <span className="editor-actions">
        <button
          type="button"
          className="tool-button"
          disabled={!canFormat || disabled}
          title="格式化 JSON"
          onClick={handleFormat}
        >
          <WandSparkles aria-hidden="true" size={16} />
        </button>
      </span>
    </label>
  );
}

