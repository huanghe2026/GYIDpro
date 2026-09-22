// 中英文切换按钮。variant="dark" 用于深色门户导航，"light" 用于白色控制台导航。
// 显示的是「切换后」的语言：中文界面显示 EN，英文界面显示 中。
import { locale, toggleLocale } from "../i18n";

export default function LangSwitch(props: { variant?: "dark" | "light" }) {
  const label = () => (locale() === "zh" ? "EN" : "中");
  const title = () =>
    locale() === "zh" ? "Switch to English" : "切换为中文";

  return (
    <button
      onClick={toggleLocale}
      title={title()}
      aria-label={title()}
      class={
        props.variant === "light"
          ? "text-xs font-semibold text-gray-600 border rounded px-2 py-0.5 hover:bg-gray-50 transition-colors"
          : "text-xs font-semibold text-slate-300 hover:text-white border border-white/15 rounded-md px-2 py-1 transition-colors"
      }
    >
      {label()}
    </button>
  );
}
