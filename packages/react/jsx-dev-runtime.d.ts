/// GPUIX JSX dev-runtime types — mirrors jsx-runtime.d.ts for development builds.

import type {
  AnchoredProps,
  BrowserSurfaceProps,
  CanvasProps,
  CodeProps,
  DiffProps,
  ImgProps,
  InputProps,
  JsxIntrinsicProps,
  MarkdownProps,
  Props,
  SvgProps,
  TerminalProps,
  TextareaProps,
  VirtualListProps,
} from "./dist/types/host"

export { jsx, jsxs, Fragment } from "react/jsx-dev-runtime"

export namespace JSX {
  type ElementType = React.JSX.ElementType
  type Element = React.JSX.Element
  type ElementClass = React.JSX.ElementClass
  type ElementAttributesProperty = React.JSX.ElementAttributesProperty
  type ElementChildrenAttribute = React.JSX.ElementChildrenAttribute
  type IntrinsicAttributes = React.JSX.IntrinsicAttributes
  type IntrinsicClassAttributes<T> = React.JSX.IntrinsicClassAttributes<T>

  interface IntrinsicElements {
    div: JsxIntrinsicProps<Props>
    text: JsxIntrinsicProps<Props>
    img: JsxIntrinsicProps<ImgProps>
    svg: JsxIntrinsicProps<SvgProps>
    canvas: JsxIntrinsicProps<CanvasProps>
    input: JsxIntrinsicProps<InputProps>
    textarea: JsxIntrinsicProps<TextareaProps>
    anchored: JsxIntrinsicProps<AnchoredProps>
    code: JsxIntrinsicProps<CodeProps>
    diff: JsxIntrinsicProps<DiffProps>
    markdown: JsxIntrinsicProps<MarkdownProps>
    terminal: JsxIntrinsicProps<TerminalProps>
    "browser-surface": JsxIntrinsicProps<BrowserSurfaceProps>
    "virtual-list": JsxIntrinsicProps<VirtualListProps>
  }
}
