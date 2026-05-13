import { Filter, Trash2 } from "lucide-react";
import { useMemo, useState } from "react";
import { safeFormatTimestamp } from "../lib/json";
import type { EventSource, RuntimeEvent } from "../types";
import { LevelDot } from "./StatusPill";

type EventFilter = "all" | EventSource | "errors";

interface EventLogProps {
  events: RuntimeEvent[];
  onClear: () => void;
}

const filterOptions: Array<{ label: string; value: EventFilter }> = [
  { label: "全部", value: "all" },
  { label: "服务端", value: "server" },
  { label: "客户端", value: "client" },
  { label: "错误", value: "errors" }
];

export function EventLog({ events, onClear }: EventLogProps) {
  const [filter, setFilter] = useState<EventFilter>("all");
  const filteredEvents = useMemo(() => {
    return events.filter((event) => {
      if (filter === "all") return true;
      if (filter === "errors") return event.level === "error" || event.level === "warning";
      return event.source === filter;
    });
  }, [events, filter]);

  return (
    <section className="event-shell" aria-label="事件流">
      <div className="section-heading">
        <div>
          <p className="eyebrow">Runtime Events</p>
          <h2>事件流</h2>
        </div>
        <div className="event-tools">
          <div className="segmented" aria-label="事件过滤">
            <Filter aria-hidden="true" size={15} />
            {filterOptions.map((option) => (
              <button
                key={option.value}
                type="button"
                className={filter === option.value ? "is-selected" : ""}
                onClick={() => setFilter(option.value)}
              >
                {option.label}
              </button>
            ))}
          </div>
          <button type="button" className="tool-button" title="清空当前事件" onClick={onClear}>
            <Trash2 aria-hidden="true" size={16} />
          </button>
        </div>
      </div>

      <div className="event-list">
        {filteredEvents.length === 0 ? (
          <div className="empty-state">暂无事件</div>
        ) : (
          filteredEvents
            .slice()
            .reverse()
            .map((event) => (
              <article key={event.id} className={`event-row event-${event.direction}`}>
                <div className="event-meta">
                  <LevelDot level={event.level} />
                  <span>{safeFormatTimestamp(event.timestamp)}</span>
                  <span>{event.source === "server" ? "服务端" : event.source === "client" ? "客户端" : "系统"}</span>
                  <span>{event.direction}</span>
                </div>
                <div className="event-title">
                  <strong>{event.title}</strong>
                  {event.peerId ? <span className="peer-tag">{shortId(event.peerId)}</span> : null}
                </div>
                {event.payload ? <pre>{prettyPayload(event.payload)}</pre> : null}
              </article>
            ))
        )}
      </div>
    </section>
  );
}

function prettyPayload(payload: string): string {
  try {
    return JSON.stringify(JSON.parse(payload), null, 2);
  } catch {
    return payload;
  }
}

function shortId(id: string): string {
  return id.length > 12 ? `${id.slice(0, 8)}...` : id;
}

