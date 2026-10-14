import {useState} from 'react';
import {importServiceConfig} from './bridge';

export function ServiceSetup({onConfigured,onCancel}:{onConfigured:()=>void;onCancel?:()=>void}) {
  const [error,setError]=useState(''),[busy,setBusy]=useState(false);
  return <div className="onboarding"><div className="onboarding-copy">
    <span className="eyebrow">WELCOME TO OPENRIDESYNC</span>
    <h1>連結服務，<br/><span>開始你的騎乘收藏。</span></h1>
    <p className="lead">先匯入你自己的 RoadSync 服務設定，<br/>接著以原本的 Google 帳號登入。</p>
  </div><section className="onboarding-panel" aria-labelledby="service-title">
    <span className="eyebrow">LOCAL SETUP</span><h2 id="service-title">匯入服務設定</h2>
    <p className="muted">此版本尚未配置服務。請選擇你有權使用的 service-config.json；格式與準備方式請參閱 README。</p>
    <label className="service-file">服務設定檔（JSON）<input className="form-control" type="file" accept=".json,application/json" disabled={busy} onChange={async event=>{
      const input=event.currentTarget,file=input.files?.[0];
      if(!file)return;
      setError('');setBusy(true);
      try {
        if(file.size>16384)throw new Error('SERVICE_CONFIG_INVALID');
        await importServiceConfig(await file.text());onConfigured();
      } catch(e) {
        const code=e instanceof Error?e.message:String(e);
        setError(code==='SERVICE_CONFIG_SIGN_OUT_FIRST'?'請先從帳號管理清除本機資料，再更換服務設定。':code==='SERVICE_CONFIG_INVALID'?'設定格式不正確，請確認 firebase 與 hondaApiKey 欄位。':'無法匯入設定，請確認檔案內容與本機權限。');
      } finally {setBusy(false);input.value='';}
    }}/></label>
    <div className="onboarding-status" role="status">{busy?'正在儲存本機設定…':error?<p className="error-text" role="alert">{error}</p>:'設定只儲存在這個 Windows 使用者的本機資料夾。'}</div>
    <p className="privacy-note">這是獨立開源應用程式，與 Honda 無官方關聯。請只匯入可信來源的設定；無需提供 Google 密碼、登入權杖或歷史旅程檔案。</p>
  </section></div>;
}
