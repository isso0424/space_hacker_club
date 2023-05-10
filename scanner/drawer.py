import csv
import matplotlib.pyplot as plt
import numpy as np

with open("systems.csv") as f:
    reader = csv.reader(f)
    data = [row for row in reader]

def getByIndex(index):
    return np.array(list(map(lambda x: int(x[index]), data)))

labels = list(map(lambda x: x[0], data))
x = list(map(lambda x: int(x[1]), data))
y = list(map(lambda x: int(x[2]), data))
markets = np.array(list(map(lambda x: int(x[3]), data)))
commonMetal = getByIndex(4)
preciousMetal = getByIndex(5)
rareMetal = getByIndex(6)
mineral = getByIndex(7)
iceCrystal = getByIndex(8)
exportsFuel = getByIndex(9)

c = markets

for i in range(4, 10):
    c = getByIndex(i)
    cs = plt.scatter(x, y, c=c, vmax=c.max(), vmin=c.min(), cmap="Wistia")
    plt.colorbar(cs)

    for i, label in enumerate(labels):
        plt.annotate(label, (x[i], y[i]))

    plt.show()
