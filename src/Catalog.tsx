import {useMemo,useState} from 'react';
import type {Snapshot} from './types';
import {modelName,modelImage,generation} from './format';
import {Icon} from './Icon';
export function Catalog({snapshot,focus,onClearFocus,onHistory}:{snapshot:Snapshot;focus:string;onClearFocus:()=>void;onHistory:(label:string)=>void}) {
  const [query,setQuery]=useState(''),[year,setYear]=useState(''),[gen,setGen]=useState('');
  const models=useMemo(()=>snapshot.models.filter(m=>(!focus||m.modelLabel===focus)&&(!query||[modelName(m),m.modelLabel,...m.vehicleCodes.flatMap(c=>[c.modelCode,c.seriesCode])].join(' ').toLowerCase().includes(query.toLowerCase()))&&(!year||m.modelYears.includes(Number(year)))&&(!gen||String(m.meter.generation)===gen)).sort((a,b)=>modelName(a).localeCompare(modelName(b))),[snapshot,query,year,gen,focus]);
  return <section className="workspace">
    <div className="workspace-heading"><div><span className="eyebrow">VEHICLE COLLECTION</span><h1>車型圖鑑<span className="count-label">{snapshot.models.length} MODELS</span></h1><p>從車型、年式到儀表資訊，找到與旅程相關的車款。</p></div><div className="section-index">01 / 03</div></div>
    <div className="filters catalog-filters">
      <label>搜尋車型<input className="form-control" type="search" value={query} placeholder="車名、型號或車端代碼" onChange={e=>{setQuery(e.target.value);onClearFocus();}}/></label>
      <label>圖鑑年式<select className="form-select" value={year} onChange={e=>{setYear(e.target.value);onClearFocus();}}><option value="">全部年式</option>{[...new Set(snapshot.models.flatMap(m=>m.modelYears))].sort((a,b)=>b-a).map(y=><option key={y}>{y}</option>)}</select></label>
      <label>儀表世代<select className="form-select" value={gen} onChange={e=>{setGen(e.target.value);onClearFocus();}}><option value="">全部世代</option><option value="1">Gen 1.0</option><option value="2">Gen 1.5</option><option value="3">Gen 2.0</option></select></label>
    </div>
    <div className="result-line"><span role="status">顯示 {models.length} 個型號</span>{focus?<button className="text-button" onClick={()=>{onClearFocus();setQuery('');setYear('');setGen('');}}>清除旅程車型篩選 · {focus} ×</button>:<span>所有可見車系 · 已保存圖片</span>}</div>
    <div className="vehicle-grid">{models.map(m=>{
      const related=snapshot.days.filter(d=>d.vehicle.models.some(c=>c.label===m.modelLabel)).length;
      return <article className="vehicle-card" key={m.modelLabel}>
        <div className="vehicle-art"><span className="model-label">{m.modelLabel}</span>{modelImage(m,snapshot.images)?<img src={modelImage(m,snapshot.images)} alt={modelName(m)} loading="lazy"/>:<span>圖片未提供</span>}<span className="vehicle-generation">{generation(m)}</span></div>
        <div className="vehicle-body"><div className="vehicle-title"><h2>{modelName(m)}</h2><span>{m.modelYears.join(' / ')}</span></div>
          <p className="muted">{m.modelSeriesLabel} · 儀表 type {m.meter.type}</p>
          <div className="vehicle-code"><span>車端代碼</span><strong>{m.vehicleCodes.map(c=>c.seriesCode+' / '+c.modelCode).join('、')||'未提供'}</strong></div>
          <details><summary>儀表語言與完整資料</summary><p className="muted">完整支援：{m.meter.fullSupportedLanguageCodes?.join('、')||'未提供'}</p><pre>{JSON.stringify(m,null,2)}</pre></details>
          {related>0?<button className="related-button" onClick={()=>onHistory(m.modelLabel)}><span>查看 {related} 個相關行程日</span><Icon kind="arrow" size={18}/></button>:<p className="no-related">尚無可對應的旅程</p>}
        </div>
      </article>;
    })}</div>{models.length===0&&<div className="empty-state"><h2>沒有符合的車型</h2><p>試試不同的車名、年式或儀表世代。</p></div>}
  </section>;
}
