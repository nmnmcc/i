from probe import case
n=4096
src='v: Int = 1\nA0 = v\n'+'\n'.join(f'A{i} = A{i-1}' for i in range(1,n))+'\nx: A'+str(n-1)+' = 2'
case('valuechain4096',src,'pass with bounded cost')
