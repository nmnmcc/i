from probe import case
for n in [128,256,512]:
 src='v: Int = 1\n'+'\n'.join(f'A{i} = A{i-1}' for i in range(n-1,0,-1))+'\nA0 = v\nx: A'+str(n-1)+' = 2'
 case('reversechain'+str(n),src,'same cost as forward alias chain')
