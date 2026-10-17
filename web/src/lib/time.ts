const rtf = new Intl.RelativeTimeFormat("zh-CN", { numeric: "auto" });

/** "刚刚", "5 分钟前", "3 天前" … for timestamps in the past. */
export function ago(input: string | number | null | undefined, now = Date.now()): string {
  if (input == null) return "";
  const time = typeof input === "number" ? input * 1000 : Date.parse(input);
  if (Number.isNaN(time)) return "";
  const seconds = Math.round((time - now) / 1000);
  const abs = Math.abs(seconds);
  if (abs < 45) return "刚刚";
  if (abs < 3600) return rtf.format(Math.round(seconds / 60), "minute");
  if (abs < 86400) return rtf.format(Math.round(seconds / 3600), "hour");
  if (abs < 86400 * 30) return rtf.format(Math.round(seconds / 86400), "day");
  if (abs < 86400 * 365) return rtf.format(Math.round(seconds / (86400 * 30)), "month");
  return rtf.format(Math.round(seconds / (86400 * 365)), "year");
}

export function startOfDay(date: Date): Date {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate());
}

export function dayDiff(due: Date, now: Date): number {
  return Math.round((startOfDay(due).getTime() - startOfDay(now).getTime()) / 86400000);
}

const weekday = new Intl.DateTimeFormat("zh-CN", { weekday: "short" });
const monthDay = new Intl.DateTimeFormat("zh-CN", { month: "numeric", day: "numeric" });
const clock = new Intl.DateTimeFormat("zh-CN", { hour: "2-digit", minute: "2-digit", hour12: false });

/** Short label for a due date relative to today. */
export function dueLabel(iso: string, now: Date): string {
  const due = new Date(iso);
  const diff = dayDiff(due, now);
  const hasTime = !(due.getHours() === 0 && due.getMinutes() === 0) && !(due.getHours() === 23 && due.getMinutes() === 59);
  const time = hasTime ? ` ${clock.format(due)}` : "";
  if (diff === 0) return hasTime ? clock.format(due) : "今天";
  if (diff === -1) return `昨天${time}`;
  if (diff < -1) return `${-diff} 天前`;
  if (diff === 1) return `明天${time}`;
  if (diff < 7) return `${weekday.format(due)}${time}`;
  return monthDay.format(due);
}

export function greeting(date: Date): string {
  const hour = date.getHours();
  if (hour < 5) return "夜深了";
  if (hour < 11) return "早上好";
  if (hour < 13) return "中午好";
  if (hour < 18) return "下午好";
  return "晚上好";
}
