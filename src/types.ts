export interface Model {
  modelLabel: string; modelSeriesLabel: string; modelYears: number[]; displacement: number;
  modelName: Record<string, {fullName?: string}>;
  meter: {generation: number; type: number; fullSupportedLanguageCodes?: string[]; partiallySupportedLanguageCodes?: string[]};
  vehicleCodes: {modelCode: string; seriesCode: string}[];
  images: {id: string; description?: {isDefault?: boolean}}[];
}
export interface Trip {distance: number; avgSpeed: number; start: number|null; end: number|null; points: [number,number,number,boolean][]; coarsePoints: number}
export interface Day {
  date: string; start: number; end: number; distance: number; duration: number; avgSpeed: number; trips: Trip[];
  vehicle: {names: string[]; codes: string[]; basis: string; nameConflict: boolean; unmatchedCodes: string[]; models: {label: string; reasons: string[]}[]};
}
export interface Snapshot {
  version: number; updatedAt: string; models: Model[]; seriesCount: number;
  images: Record<string,string>; days: Day[]; summary: {days: number; trips: number; points: number};
  cacheState?:{imagesCleared?:boolean;historyCleared?:boolean};
}
export interface AccountInfo {identity:{uid:string;email:string|null;name:string|null}|null;identityUnavailable:boolean;hasSavedSession:boolean;images:number;days:number;trips:number}
export interface DownloadProgress {completed:number;total:number|null;message:string}
export interface Progress {phase: string; message: string; busy: boolean; error: string|null; loginUrl: string|null; session:'none'|'checking'|'active'|'expired'|'offline';catalog:DownloadProgress;history:DownloadProgress}
