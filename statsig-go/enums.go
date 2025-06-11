package statsig

type HashAlgo string

const (
	DJB2   HashAlgo = "Djb2"
	SHA256 HashAlgo = "Sha256"
	NONE   HashAlgo = "None"
)

type GCIRResponseFormat string

const (
	Initialize                             GCIRResponseFormat = "v1"
	InitializeWithSecondaryExposureMapping GCIRResponseFormat = "v2"
)
