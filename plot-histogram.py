import matplotlib.pyplot as plt
import numpy as np
 
with open('noise-no-capacitor', 'r') as f:
    no_capa = [int(i.strip()) for i in f]
with open('noise-with-capacitor', 'r') as f:
    with_capa = [int(i.strip()) for i in f]

no_capa_range = max(no_capa) - min(no_capa) 
with_capa_range = max(with_capa) - min(with_capa) 

# plotting first histogram
plt.hist(no_capa, bins=no_capa_range, color="blue")
 
# plotting second histogram
plt.hist(with_capa, bins=with_capa_range, color="red")
plt.legend(['no capa', 'with capa'])
 
# Showing the plot using plt.show()
plt.show()

def stats(v, n):
    std_dev = np.std(v)
    avg = np.average(v)
    print(f"{n}: {avg}+-{std_dev}")

stats(no_capa, "no_capa")
stats(with_capa, "with_capa")