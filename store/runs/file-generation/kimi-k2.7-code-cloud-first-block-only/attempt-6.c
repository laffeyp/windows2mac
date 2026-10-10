if (existing->access & READ) and (new_access & WRITE) and !(new_share & WRITE) -> conflict
if (existing->access & WRITE):
  if (new_access & READ) and !(new_share & READ) -> conflict
  if (new_access & WRITE) and !(new_share & WRITE) -> conflict
if (new_access & READ) and (existing->access & WRITE) and !(existing->share & READ) -> conflict
if (new_access & WRITE):
  if (existing->access & READ) and !(existing->share & WRITE) -> conflict
  if (existing->access & WRITE) and !(existing->share & WRITE) -> conflict
