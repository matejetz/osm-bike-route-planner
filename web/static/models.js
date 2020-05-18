let EDGE_COLORS = ['black','red','green','blue','orange','yellow', 'purple', 'pink', 'gold', 'tomato', 'olivedrab'];
let edge_color_count = 0;

function get_next_edge_color() {
    // looping over edge colors
    return EDGE_COLORS[edge_color_count++ % EDGE_COLORS.length]
}

class Result {
    weightedResults = [];
    _distanceType;

    add(result) {
        result.color = get_next_edge_color();
        this.weightedResults.push(result);
        this._distanceType = result.distance_type;
    }

    get distanceType() {
        return this._distanceType
    }

    get minElevation() {
        return Math.min(...this.weightedResults.map(result => result.elevation))
    }

    get maxElevation() {
        return Math.max(...this.weightedResults.map(result => result.elevation))
    }

    get minDistance() {
        return Math.min(...this.weightedResults.map(result => result.distance))
    }

    get maxDistance() {
        return Math.max(...this.weightedResults.map(result => result.distance))
    }
}