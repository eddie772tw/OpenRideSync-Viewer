export function filterDays(days, {query='', model='', from='', to=''}) {
  const q=query.trim().toLowerCase();
  return days.filter(d=>(!q||[...d.vehicle.names,...d.vehicle.codes].join(' ').toLowerCase().includes(q))
    &&(!model||(model==='unmatched'?!d.vehicle.models.length:d.vehicle.models.some(m=>m.label===model)))
    &&(!from||d.date>=from)&&(!to||d.date<=to));
}
