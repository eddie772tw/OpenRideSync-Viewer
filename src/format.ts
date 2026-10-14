import type { Model } from './types';
export const numeral = new Intl.NumberFormat('zh-TW',{maximumFractionDigits:1});
export const km = (value:number) => numeral.format(value/1000);
export const speed = (value:number) => numeral.format(value*3.6);
export const duration = (seconds:number) => Math.floor(seconds/3600)+' 小時 '+Math.floor(seconds%3600/60)+' 分';
export const time = (value:number|null) => value ? new Date(value*1000).toLocaleTimeString('zh-TW',{timeZone:'Asia/Taipei',hour12:false,hour:'2-digit',minute:'2-digit'}) : '未提供';
export const modelName = (model:Model) => model.modelName?.displayName?.fullName || model.modelName?.defaultName?.fullName || model.modelLabel;
export const generation = (model:Model) => ({1:'Gen 1.0',2:'Gen 1.5',3:'Gen 2.0'}[model.meter.generation] || '未提供');
export const modelImage = (model:Model,images:Record<string,string>) => images[(model.images.find(i=>i.description?.isDefault)||model.images[0])?.id];
