/// GPUIX JSX dev-runtime types — mirrors jsx-runtime.d.ts for development builds.

import type {
  AnchoredProps,
  CanvasProps,
  CodeProps,
  DiffProps,
  ImgProps,
  InputProps,
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
    div: Props
    text: Props
    img: ImgProps
    svg: SvgProps
    canvas: CanvasProps
    input: InputProps
    textarea: TextareaProps
    anchored: AnchoredProps
    code: CodeProps
    diff: DiffProps
    markdown: MarkdownProps
    terminal: TerminalProps
    "virtual-list": VirtualListProps
  }
}
