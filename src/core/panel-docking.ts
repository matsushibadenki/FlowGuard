import type {Locale} from './types';
export const panelDragType='application/x-flowguard-panel';
export const panelDragValue='inspector:v1';
export function acceptsPanel(types:readonly string[],value?:string){return types.includes(panelDragType)&&(value===undefined||value===panelDragValue);}
export const dockingWords:Record<Locale,{drag:string;left:string;right:string;float:string}>={ja:{drag:'詳細パネルを移動（←左、→右、Enterで独立）',left:'左へドッキング',right:'右へドッキング',float:'独立ウインドウ'},en:{drag:'Move details panel (← left, → right, Enter to float)',left:'Dock left',right:'Dock right',float:'Float window'},'zh-CN':{drag:'移动详情面板（←左，→右，Enter独立窗口）',left:'停靠左侧',right:'停靠右侧',float:'独立窗口'}};
