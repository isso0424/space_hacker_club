import csv
import matplotlib.pyplot as plt
import numpy as np

with open("systems.csv") as f:
    reader = csv.reader(f)
    data = [row for row in reader]

labels = list(map(lambda x: x[0], data))
x = list(map(lambda x: int(x[1]), data))
y = list(map(lambda x: int(x[2]), data))
c = np.array(list(map(lambda x: int(x[3]), data)))

cs = plt.scatter(x, y, c=c, vmax=c.max(), vmin=c.min(), cmap="Wistia")
plt.colorbar(cs)

for i, label in enumerate(labels):
    plt.annotate(label, (x[i], y[i]))

plt.show()
