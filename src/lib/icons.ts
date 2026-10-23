// The app's icons: a hand-picked subset of Lucide (https://lucide.dev, ISC),
// imported one by one so only these end up in the bundle.

import {
  BookOpen, Copy, CornerDownRight, FileCode, FolderInput, GitBranch, GitMerge, Globe, Link, MessageSquare, Ellipsis, Pencil,
  Archive, ArrowDown, ArrowUp, Bookmark, Bug, Calendar, Check, ChevronDown, ChevronLeft, ChevronRight,
  Circle, CircleAlert, CircleCheck, CircleDashed, CircleDot, CircleHelp, CirclePause, CirclePlay, CircleSlash,
  CircleX, Clock, Diamond, ExternalLink, Eye, FilePlus, FileText, Flag, Flame, GitPullRequest, Hourglass,
  ImagePlus, Inbox, Info, Keyboard, List, Menu, Pin, PinOff, Plus, Search, SearchX, Settings, Square, Star, Ticket,
  Tag, Tags, Trash2, Triangle, TriangleAlert, X, Zap, type IconNode,
} from "lucide";

const ICONS = {
  ellipsis: Ellipsis,
  pencil: Pencil,
  "folder-input": FolderInput,
  "image-plus": ImagePlus,
  archive: Archive,
  "arrow-down": ArrowDown,
  "arrow-up": ArrowUp,
  bookmark: Bookmark,
  "book-open": BookOpen,
  copy: Copy,
  "corner-down-right": CornerDownRight,
  "file-code": FileCode,
  "git-branch": GitBranch,
  "git-merge": GitMerge,
  globe: Globe,
  link: Link,
  "message-square": MessageSquare,
  bug: Bug,
  calendar: Calendar,
  check: Check,
  "chevron-down": ChevronDown,
  "chevron-left": ChevronLeft,
  "chevron-right": ChevronRight,
  circle: Circle,
  "circle-alert": CircleAlert,
  "circle-check": CircleCheck,
  "circle-dashed": CircleDashed,
  "circle-dot": CircleDot,
  "circle-question-mark": CircleHelp,
  "circle-pause": CirclePause,
  "circle-play": CirclePlay,
  "circle-slash": CircleSlash,
  "circle-x": CircleX,
  clock: Clock,
  diamond: Diamond,
  "external-link": ExternalLink,
  eye: Eye,
  "file-plus": FilePlus,
  "file-text": FileText,
  flag: Flag,
  flame: Flame,
  "git-pull-request": GitPullRequest,
  hourglass: Hourglass,
  inbox: Inbox,
  info: Info,
  keyboard: Keyboard,
  list: List,
  menu: Menu,
  pin: Pin,
  "pin-off": PinOff,
  plus: Plus,
  search: Search,
  "search-x": SearchX,
  settings: Settings,
  square: Square,
  tag: Tag,
  tags: Tags,
  star: Star,
  ticket: Ticket,
  trash: Trash2,
  triangle: Triangle,
  "triangle-alert": TriangleAlert,
  x: X,
  zap: Zap,
} satisfies Record<string, IconNode>;

export type IconName = keyof typeof ICONS;

/** Icons offered for statuses in Settings, in picker order. */
export const STATUS_ICONS: IconName[] = [
  "circle", "circle-dot", "circle-dashed", "circle-check", "circle-slash", "circle-x",
  "circle-pause", "circle-play", "circle-alert", "circle-question-mark", "clock", "hourglass",
  "eye", "flag", "star", "bookmark", "zap", "flame", "bug", "diamond", "square", "triangle", "archive", "inbox",
];

/** The icon's shapes; unknown names (e.g. a typo in settings.json) get a plain circle. */
/** Icons offered for ticket systems and link types. */
export const LINK_ICONS: IconName[] = [
  "ticket", "bug", "circle-dot", "flag", "git-pull-request", "git-merge", "git-branch",
  "book-open", "file-text", "file-code", "link", "globe", "message-square", "star",
];

export function iconNode(name: string): IconNode {
  return (ICONS as Record<string, IconNode>)[name] ?? Circle;
}

export function isIcon(name: string): name is IconName {
  return name in ICONS;
}

const SVG = "http://www.w3.org/2000/svg";

/** An icon as a DOM element, for code that builds DOM by hand (editor widgets). */
export function iconElement(name: string, size = 14): SVGSVGElement {
  const svg = document.createElementNS(SVG, "svg");
  for (const [k, v] of Object.entries(svgAttrs(size))) svg.setAttribute(k, String(v));
  for (const [tag, attrs] of iconNode(name)) {
    const el = document.createElementNS(SVG, tag);
    for (const [k, v] of Object.entries(attrs)) if (v !== undefined) el.setAttribute(k, String(v));
    svg.append(el);
  }
  return svg;
}

export function svgAttrs(size: number) {
  return {
    class: "icon",
    width: size,
    height: size,
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: "currentColor",
    "stroke-width": 2,
    "stroke-linecap": "round",
    "stroke-linejoin": "round",
    "aria-hidden": "true",
  } as const;
}
