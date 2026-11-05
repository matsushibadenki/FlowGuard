import type {ManualCheck} from './types';

// Compare calendar dates in the device's local time, without interpreting a date as UTC.
export function localReviewDate(date:Date):string{
 const year=String(date.getFullYear()).padStart(4,'0');
 return `${year}-${String(date.getMonth()+1).padStart(2,'0')}-${String(date.getDate()).padStart(2,'0')}`;
}
function validDate(value:string):boolean{
 const match=/^(\d{4})-(\d{2})-(\d{2})$/.exec(value);
 if(!match)return false;
 const year=Number(match[1]),month=Number(match[2]),day=Number(match[3]);
 if(year<1||month<1||month>12)return false;
 const leap=year%4===0&&(year%100!==0||year%400===0);
 const days=[31,leap?29:28,31,30,31,30,31,31,30,31,30,31];
 return day>=1&&day<=days[month-1];
}
export function manualReviewDue(entry:Pick<ManualCheck,'status'|'nextAt'>|undefined,today:string):'overdue'|'today'|null{
 if(!entry||!['confirmed','needs_work'].includes(entry.status)||!validDate(entry.nextAt)||!validDate(today))return null;
 return entry.nextAt<today?'overdue':entry.nextAt===today?'today':null;
}

export function exposureReviewDue(entry:{status:string;nextAt?:string}|undefined,today:string):'overdue'|'today'|null{
 if(!entry||!['expected','unexpected'].includes(entry.status)||!entry.nextAt||!validDate(entry.nextAt)||!validDate(today))return null;
 return entry.nextAt<today?'overdue':entry.nextAt===today?'today':null;
}
