import { invoke, isTauri } from '@tauri-apps/api/core';
import type { AccountInfo, Progress, Snapshot } from './types';
export async function serviceConfigured():Promise<boolean>{
  if(isTauri())return invoke('service_configured');
  return !new URLSearchParams(location.search).has('setup');
}
export async function importServiceConfig(text:string):Promise<void>{
  if(!isTauri())throw new Error('DESKTOP_APP_REQUIRED');
  return invoke('import_service_config',{text});
}
// Browser preview reads an explicitly prepared, ignored fixture. Release has no HTTP data API.
export async function loadSnapshot():Promise<Snapshot|null> {
  if(isTauri()) return invoke('load_snapshot');
  if(import.meta.env.DEV) {
    const onboarding=new URLSearchParams(location.search).get('onboarding');
    if(onboarding!==null&&onboarding!=='ready') return null;
    const r=await fetch('/__preview/snapshot');
    return r.ok ? r.json() : null;
  }
  return null;
}
export async function status():Promise<Progress> {
  if(isTauri()) return invoke('viewer_status');
  const params=new URLSearchParams(import.meta.env.DEV?location.search:'');
  const base:Progress={phase:'idle',message:'',busy:false,error:null,loginUrl:null,session:'none',catalog:{completed:0,total:null,message:''},history:{completed:0,total:null,message:''}};
  const onboarding=params.get('onboarding');
  if(onboarding==='download')return {...base,phase:'download',message:'車型與旅程同時下載中',busy:true,session:'active',catalog:{completed:42,total:58,message:'車型圖片 · 42 / 58 張'},history:{completed:29,total:71,message:'旅程路線 · 29 / 71 日'}};
  if(onboarding==='ready')return {...base,phase:'done',message:'車型與旅程已準備完成',session:'active'};
  const mode=params.get('session');
  return mode==='expired'?{...base,phase:'error',message:'登入已失效',error:'SESSION_EXPIRED',session:'expired'}:
    mode==='offline'?{...base,phase:'error',message:'暫時無法連線',error:'SESSION_OFFLINE',session:'offline'}:base;
}
export async function startSync(forceLogin=false):Promise<void> {
  if(!isTauri()) throw new Error('DESKTOP_APP_REQUIRED');
  await invoke('start_sync',{forceLogin});
}
export async function accountInfo():Promise<AccountInfo>{
  if(isTauri())return invoke('account_info');
  const cached=await loadSnapshot();
  return {identity:{uid:'preview-account-id',email:'rider@example.test',name:'展示帳號'},identityUnavailable:false,hasSavedSession:true,images:Object.keys(cached?.images||{}).length,days:cached?.summary.days||0,trips:cached?.summary.trips||0};
}
export async function accountAction(action:string):Promise<void>{
  if(!isTauri())throw new Error('瀏覽器預覽不會修改本機帳號或快取，請使用桌面應用程式。');
  return invoke('account_action',{action});
}
