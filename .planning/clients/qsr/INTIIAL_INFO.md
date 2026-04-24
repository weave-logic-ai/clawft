We are looking to understand and ensure we can build a system for QSR that would be able to take thousands of stores' daily processing from a series of large fast-food restaurants, all owned by the same company. They have the data in data lakes already, so we will be able to pull it. The important part here is to look at how, in WeftOS, we can model this data daily for them using ECC so they can ask questions like "if the stores in Metro-Alpha don't meet their budget this week, what will their quarterly revenue look like?" and things of that nature. They will have all the data we need to do this. We will need to build the models, but there will be a significant amount, so we need to figure out the upper limits of what we can shove into RVF format and how we will keep this.

Scale, in rough order of magnitude:

- ~$125M daily sales ÷ ~$11 average order ≈ **~11M orders per day**.
- Annual system-wide sales in the tens of billions, across tens of thousands of restaurants in 100+ countries.
- Four brands under the QSR umbrella (referenced here as Brand-A through Brand-D).

QSR will need all figures from the stores. That includes sales, COGS, labor, waste, inventory, and promo activity, plus a long tail of smaller per-store signals. For each store we will also want the org chart — employees, positions, reporting lines — so we can spot gaps in who should be filling which role. It is going to be a large instance, so we will probably need to figure out partitioning carefully. We will almost certainly need a test harness that can exercise this at scale before touching any real client data.
