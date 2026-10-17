export interface Rect{x:number;y:number;width:number;height:number}
export interface GpuBounds{rect:Rect;clip:Rect;viewportWidth:number;viewportHeight:number}
export interface GpuStatus{lease:number;presented:boolean;routes:number;total:number;truncated:boolean}
/** Clip the native surface to its DOM slot, scroll containers, viewport and sticky protection bar. */
export function clipBounds(rect:Rect,viewportWidth:number,viewportHeight:number,clips:Rect[]=[]):GpuBounds|null{
 if(![rect.x,rect.y,rect.width,rect.height,viewportWidth,viewportHeight].every(Number.isFinite)||rect.width<1||rect.height<1||rect.width>4096||rect.height>4096||viewportWidth<1||viewportHeight<1)return null;
 let left=Math.max(0,rect.x),top=Math.max(0,rect.y),right=Math.min(viewportWidth,rect.x+rect.width),bottom=Math.min(viewportHeight,rect.y+rect.height);
 for(const c of clips){left=Math.max(left,c.x);top=Math.max(top,c.y);right=Math.min(right,c.x+c.width);bottom=Math.min(bottom,c.y+c.height);}
 if(right-left<1||bottom-top<1)return null;
 return {rect,clip:{x:left,y:top,width:right-left,height:bottom-top},viewportWidth,viewportHeight};
}
export const gpuWords={
 ja:{title:'大規模マップ',open:'大規模マップを開く',close:'大規模マップを閉じる',apps:'アプリ',destinations:'接続先',waiting:'描画待ち',failed:'描画できません。通常の一覧を利用できます。',unavailable:'ネイティブ描画は、この機能を含むmacOSデスクトップ版で利用できます。',help:'Rustが保持する全接続を最大10,000件描画します。この概要は検索・ループバック非表示の対象外です。左がアプリ、右がIPとポート。青は選択中、灰は観測した接続です。線は許可や遮断の証拠ではありません。アプリ名・接続先の詳細と操作は下の通常表示を使います。描画データはRust／GPU内に保持します。',routes:'描画',total:'全接続',limit:'上限により一部省略'},
 en:{title:'Dense map',open:'Open dense map',close:'Close dense map',apps:'Applications',destinations:'Destinations',waiting:'Waiting for rendering',failed:'Rendering unavailable. The standard list remains available.',unavailable:'Native rendering requires the macOS desktop build with this feature.',help:'Draws up to 10,000 Rust-owned connections. This overview includes all connections, independent of search and loopback filters. Applications are on the left; IP and port destinations are on the right. Blue means selected; gray means observed. Lines do not prove allow or block enforcement. Use the standard view below for names, details and actions. Render data stays in Rust/GPU memory.',routes:'Drawn',total:'Total connections',limit:'Some connections omitted at the limit'},
 'zh-CN':{title:'大规模地图',open:'打开大规模地图',close:'关闭大规模地图',apps:'应用',destinations:'连接目标',waiting:'等待绘制',failed:'无法绘制，可继续使用常规列表。',unavailable:'原生绘制需要包含此功能的macOS桌面版。',help:'绘制Rust持有的全部连接，最多10,000条。此概览不受搜索和隐藏回环选项影响。左侧为应用，右侧为IP和端口。蓝色表示选中，灰色表示观测到的连接。线条不能证明允许或阻止已生效。名称、详情及操作请使用下方常规视图。绘制数据保留在Rust／GPU内存中。',routes:'已绘制',total:'全部连接',limit:'达到上限，部分连接已省略'}
};
