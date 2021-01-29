const macroRegex = /html! {([\s\S]*?)}/g;

export function isInsideHtmlMacro(
  match: RegExpMatchArray,
  cursor: number
): boolean {
  return false;
}
