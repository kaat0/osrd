import { useContext, useEffect, useMemo, useState } from 'react';

import { useTranslation } from 'react-i18next';
import { useSelector } from 'react-redux';

import type { Comfort } from 'common/api/osrdEditoastApi';
import { ModalContext } from 'common/BootstrapSNCF/ModalSNCF/ModalProvider';
import OptionsSNCF from 'common/BootstrapSNCF/OptionsSNCF';
import type { Option } from 'common/BootstrapSNCF/OptionsSNCF';
import { useOsrdConfActions } from 'common/osrdContext';
import { comfort2pictogram } from 'modules/rollingStock/components/RollingStockSelector/RollingStockHelpers';
import { updateRollingStockComfort } from 'reducers/osrdconf/operationalStudiesConf';
import { getRollingStockComfort } from 'reducers/osrdconf/operationalStudiesConf/selectors';
import { useAppDispatch } from 'store';

interface RollingStockCardButtonsProps {
  id: number;
  curvesComfortList: Comfort[];
  setOpenedRollingStockCardId: (openCardId: number | undefined) => void;
}

const RollingStockCardButtons = ({
  id,
  curvesComfortList,
  setOpenedRollingStockCardId,
}: RollingStockCardButtonsProps) => {
  const dispatch = useAppDispatch();
  const { t } = useTranslation(['rollingstock']);
  const { closeModal } = useContext(ModalContext);

  const currentComfortInStore = useSelector(getRollingStockComfort);
  const [comfort, setComfort] = useState<string>(currentComfortInStore);

  const { updateRollingStockID } = useOsrdConfActions();

  const selectRollingStock = () => {
    setOpenedRollingStockCardId(undefined);
    dispatch(updateRollingStockID(id));
    dispatch(updateRollingStockComfort(comfort as Comfort));
    closeModal();
  };

  const comfortOptions = useMemo(() => {
    const options: Option[] = [{ value: 'STANDARD', label: t('comfortTypes.STANDARD') }];
    if (curvesComfortList.includes('HEATING')) {
      options.push({
        value: 'HEATING',
        label: (
          <span
            data-testid="comfort-heating-button"
            className="rollingstock-footer-button-with-picto"
          >
            {comfort2pictogram('HEATING')} {t('comfortTypes.HEATING')}
          </span>
        ),
      });
    }
    if (curvesComfortList.includes('AIR_CONDITIONING')) {
      options.push({
        value: 'AIR_CONDITIONING',
        label: (
          <span data-testid="comfort-ac-button" className="rollingstock-footer-button-with-picto">
            {comfort2pictogram('AIR_CONDITIONING')} {t('comfortTypes.AIR_CONDITIONING')}
          </span>
        ),
      });
    }
    return options;
  }, [curvesComfortList]);

  useEffect(() => {
    if (curvesComfortList.length === 0) {
      setComfort('STANDARD');
    } else {
      setComfort(
        curvesComfortList.includes(currentComfortInStore)
          ? currentComfortInStore
          : curvesComfortList[0]
      );
    }
  }, [curvesComfortList, currentComfortInStore]);

  return (
    <div className="rollingstock-footer-buttons">
      {curvesComfortList.length > 1 && (
        <OptionsSNCF
          onChange={(e) => setComfort(e.target.value)}
          options={comfortOptions}
          name="comfortChoice"
          selectedValue={comfort}
          sm
        />
      )}
      <button type="button" className="ml-2 btn btn-primary btn-sm" onClick={selectRollingStock}>
        {t('selectRollingStock')}
      </button>
    </div>
  );
};

export default RollingStockCardButtons;
